/*extern crate nalgebra_glm;
extern crate bitflags;*/

use crate::interface::{CallbackFn, EventCtx, AppState};
use crate::render_text::{TextParams};
use crate::textedit::{TextBox};
use crate::primitives::{DrawCtx, Point, Rect, RotateRect, Radians, Border, BorderRect, InBounds, rgb_to_f32};
use sdl2::mouse::SystemCursor;
use sdl2::keyboard::Keycode;
use nalgebra_glm as glm;
use bitflags::bitflags;
use std::rc::Rc;
use std::cell::RefCell;
use chrono::Datelike;

pub struct MDTitle {
    pub symbol: String,
    pub strategy: String,
    pub date: String
}

impl MDTitle {
    pub fn empty() -> Self {
        MDTitle { symbol: String::new(), strategy: String::new(), date: String::new() }
    }
}

pub struct MDDoc {
    pub title: MDTitle,
    pub portfolio: String,
    pub body: Vec<u8>
}

impl MDDoc {
    pub fn empty() -> Self {
        MDDoc { title: MDTitle::empty(), portfolio: String::new(), body: Vec::new() }
    }
}

#[derive(Debug)]
pub enum Orientation {
    Vertical,
    Horizontal
}

pub trait Widget {
    fn draw(&self, offset: &Point, ctx: &DrawCtx);
    fn measure(&self, _: &DrawCtx) -> Point {
        Point::origin()
    }
    fn hover(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> {
        None
    }
    fn click(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> {
        None
    }
    fn deselect(&mut self) -> Option<WidgetResponse> { 
        None
    }
    fn selection(&mut self) -> Option<&mut SelectionItem> {
        None
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.measure(ctx)
    }
    fn do_serialize(&self) -> bool { true }
    fn serialize(&self, _: &mut MDDoc) { }
}

bitflags! {
    pub struct WidgetStatus: u32 {
        const FINE = 0;
        const REDRAW = 1;
        const REMEASURE = 3;
    }
}

pub type WidgetResponse = (WidgetStatus, CallbackFn);

pub fn no_cb() -> CallbackFn {
    Rc::new(|_: &mut AppState| {})
}
pub fn just_cb(cb: CallbackFn) -> WidgetResponse {
    (WidgetStatus::FINE, cb)
}
pub fn just_status(status: WidgetStatus) -> WidgetResponse {
    (status, no_cb())
}

pub trait CombineResponse {
    fn combine(self, other: Self) -> Self where Self: Sized;
}

impl CombineResponse for Option<WidgetResponse> {
    fn combine(self, other: Self) -> Self {
        match self {
            Some(r) => match other {
                Some(r2) => {
                    Some(combine_response(&r, &r2))
                }
                None => Some(r) 
            },
            None => other 
        }
    }
}

pub fn combine_response(r1: &WidgetResponse, r2: &WidgetResponse) -> WidgetResponse {
    let cb1 = Rc::clone(&r1.1);
    let cb2 = Rc::clone(&r2.1);
    (r1.0 | r2.0, 
      Rc::new(move |app: &mut AppState| {
          (cb1)(app);
          (cb2)(app);
      })
    )
}

/*pub fn combine_response_opt(r1: &Option<WidgetResponse>, r2: &Option<WidgetResponse>) {
    match r1 {
        Some(w) => 
    }
}*/

pub struct WidgetList {
    pub orientation: Orientation,
    pub spacing: u32,
    pub size: Point,
    widgets: Vec<Box<dyn Widget>>,
    widget_rects: Vec<Rect>,
    needs_draw: RefCell<Vec<bool>>,
    select_list: SelectionLinkedList
}

impl WidgetList {
    pub fn new(orientation: Orientation, spacing: u32) -> Self {
        WidgetList { orientation, spacing, widgets: Vec::new(), widget_rects: Vec::new(), needs_draw: RefCell::new(Vec::new()), 
                     size: Point::origin(), select_list: SelectionLinkedList::new() }
    }
    
    pub fn get_widget(&self, idx: usize) -> Option<&Box<dyn Widget>> {
        self.widgets.get(idx)
    }
    pub fn get_widget_mut(&mut self, idx: usize) -> Option<&mut Box<dyn Widget>> {
        self.widgets.get_mut(idx)
    }
    pub fn get_idx(&self, off_pt: &Point, ctx: &DrawCtx) -> Option<usize> {
        self.widget_rects.iter().position(|r| r.in_bounds(off_pt, &ctx.viewport)) 
    }
}

impl WidgetIterT for WidgetList {
    type Child = Box<dyn Widget>;
    fn add(&mut self, mut item: Self::Child, ctx: &DrawCtx) {
        let m = item.measure(ctx);
        let spacing = if self.widgets.is_empty() { 0. } else { self.spacing as f32 };
        if let Some(select) = item.selection() {
            self.select_list.add(select);
        }
        self.widgets.push(item);
        let mut c1 = Point::origin();
        match self.orientation {
            Orientation::Vertical => {
                c1.y = self.size.y + spacing;
                self.size.x = self.size.x.max(m.x);
                self.size.y += spacing + m.y;
            }
            _ => {
                c1.x = self.size.x + spacing;
                self.size.x += spacing + m.x;
                self.size.y = self.size.y.max(m.y);
            }
        };
        self.needs_draw.borrow_mut().push(true);
        self.widget_rects.push(Rect {c1, c2: c1 + m});
    }
    fn measure_items(&self, _: &DrawCtx) -> Point {
        self.size
    }
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point {
        let mut off = Point::origin();
        let mut size = Point::origin();
        for (i, w) in self.widgets.iter_mut().enumerate() {
            let spacing = if i == 0 { 0. } else { self.spacing as f32 };
            let m = w.remeasure(ctx);
            match self.orientation {
                Orientation::Vertical => {
                    off.y = size.y + spacing;
                    size.x = size.x.max(m.x);
                    size.y += m.y + spacing;
                }
                _ => {
                    off.x = size.x + spacing;
                    size.x += m.x + spacing;
                    size.y = size.y.max(m.y);
                }
            }
            self.widget_rects[i] = Rect{ c1: off, c2: off + m };
        }
        *self.needs_draw.borrow_mut() = vec![true; self.widgets.len()];
        self.size = size;
        size
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=&'a mut Box<dyn Widget>> + 'a> {
        Box::new(self.widgets.iter_mut())
    }
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(self.widgets.iter().zip(self.widget_rects.iter()))
    }
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        Box::new(self.widgets.iter_mut().zip(self.widget_rects.iter_mut()))
    }
    fn serialize_items(&self, buf: &mut MDDoc) {
        match self.orientation {
            Orientation::Vertical => {
                for w in &self.widgets {
                    if w.do_serialize() {
                        buf.body.push('*' as u8);
                        buf.body.push(' ' as u8);
                        w.serialize(buf);
                        buf.body.push('\n' as u8);
                    }
                }
            }
            Orientation::Horizontal => {
                for w in &self.widgets {
                    if w.do_serialize() {
                        w.serialize(buf);
                        buf.body.push(' ' as u8);
                    }
                }
            }
        }
    }
    fn select_first(&mut self) -> Option<&mut SelectionItem> {
        self.select_list.head.as_mut()
    }
}

pub trait WidgetIterT {
    type Child;
    fn add(&mut self, item: Self::Child, ctx: &DrawCtx); 
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=&'a mut Box<dyn Widget>> + 'a>;
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a>;
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a>;
    fn measure_items(&self, ctx: &DrawCtx) -> Point;
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point;
    fn serialize_items(&self, buf: &mut MDDoc);
    fn select_first(&mut self) -> Option<&mut SelectionItem> { None }
    fn deselect_self(&mut self) -> Option<WidgetResponse> { None }
    fn click_self(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> { None }
    fn hover_self(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> { None }
    fn draw_self(&self, _: &Point, _: &DrawCtx) { }
    fn builder<'a>(self, ctx: &'a DrawCtx) -> WidgetBuilder<'a, Self> where Self: std::marker::Sized + 'static {
        WidgetBuilder::new(self, ctx)
    }
    fn handle_response(&mut self, resp: Option<WidgetResponse>) -> Option<WidgetResponse> {
        resp
    }
}

pub struct WidgetBuilder<'a, T: WidgetIterT> {
    w: T,
    ctx: &'a DrawCtx
}

impl<'a, T: WidgetIterT + 'static> WidgetBuilder<'a, T> {
    pub fn new(w: T, ctx: &'a DrawCtx) -> Self {
        Self { w, ctx }
    }
    pub fn get(self) -> T {
        self.w
    }
    pub fn widget(self) -> Box<dyn Widget> {
        Box::new(self.w)
    }
}

impl<'a, C, T: WidgetIterT<Child = C>> std::ops::Add<C> for WidgetBuilder<'a, T> {
    type Output = Self;
    fn add(self, c: C) -> Self {
        let mut w= self.w;
        w.add(c, self.ctx);
        WidgetBuilder { w, ctx: self.ctx }
    }
}

impl<'a, C, T: WidgetIterT<Child = C>> std::ops::AddAssign<C> for WidgetBuilder<'a, T> {
    fn add_assign(&mut self, c: C) {
        self.w.add(c, self.ctx);
    }
}

impl<T: WidgetIterT> Widget for T {
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        self.draw_self(offset, ctx);
        for (w, r) in self.widgets_plus_rects() {
            w.draw(&(*offset + r.c1), ctx);
        }
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        self.widgets_mut().fold(None, |resp, w| { 
            resp.combine(w.deselect())
        })
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let mut resp = self.click_self(off_pt, ctx);
        for (_, (w, r)) in self.widgets_plus_rects_mut().enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = resp.combine(w.click(&(*off_pt - r.c1), ctx));
                break;
            }
        }
        self.handle_response(resp)
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let mut resp = self.hover_self(off_pt, ctx);
        for (_, (w, rect)) in self.widgets_plus_rects_mut().enumerate() {
            if rect.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = resp.combine(w.hover(&(*off_pt - rect.c1), ctx));
                break;
            }
        }
        self.handle_response(resp)
    }
    fn measure(&self, ctx: &DrawCtx) -> Point {
        self.measure_items(ctx)
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.remeasure_items(ctx)
    }
    fn serialize(&self, buf: &mut MDDoc) {
        self.serialize_items(buf)
    }
    fn selection(&mut self) -> Option<&mut SelectionItem> {
        self.select_first()
    }
}

pub struct WidgetGrid {
    rows: Vec<Vec<Box<dyn Widget>>>,
    widget_rects: Vec<Vec<Rect>>,
    spacing: Point,
    size: Point,
    pub select_list: SelectionLinkedList
}

impl WidgetGrid {
    pub fn new(spacing: Point) -> Self {
        WidgetGrid { rows: Vec::new(), widget_rects: Vec::new(), spacing, size: Point::origin(), select_list: SelectionLinkedList::new() }
    }
}

impl WidgetIterT for WidgetGrid {
    type Child = Vec<Box<dyn Widget>>;
    fn add(&mut self, mut new_row: Self::Child, ctx: &DrawCtx) {
        let max_n_col = self.rows.iter().max_by_key(|r| r.len()).map(|r| r.len()).unwrap_or(0)
            .max(new_row.len());
        let mut max_col_widths: Vec<f32> = vec![0.; max_n_col];
        for row in self.widget_rects.iter() {
            for (c, rect) in row.iter().enumerate() {
                max_col_widths[c] = max_col_widths[c].max(rect.width());
            }
        }
        let mut new_rects = Vec::new();
        let mut max_height: f32 = 0.;
        for (i, w) in new_row.iter_mut().enumerate() {
            let m = w.measure(ctx);
            max_col_widths[i] = max_col_widths[i].max(m.x);
            let offset = Point::new(0., self.size.y);
            let new_rect = Rect { c1: offset, c2: offset + m };
            new_rects.push(new_rect);
            max_height = max_height.max(m.y);
            if let Some(select) = w.selection() {
                self.select_list.add(select);
            }
        }
        self.widget_rects.push(new_rects);
        self.size.y += max_height + self.spacing.y;
        for row in self.widget_rects.iter_mut() {
            let mut col_offset = 0.;
            for (c, rect) in row.iter_mut().enumerate() {
                rect.set_offset(&Point::new(col_offset, rect.c1.y));
                col_offset += max_col_widths[c] + self.spacing.x;
                self.size.x = self.size.x.max(col_offset);
            }
        }
        self.rows.push(new_row);
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>)> + 'a> {
        Box::new(self.rows.iter_mut().flatten())
    }
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(self.rows.iter().flatten().zip(self.widget_rects.iter().flatten()))
    }
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        Box::new(self.rows.iter_mut().flatten().zip(self.widget_rects.iter_mut().flatten()))
    }
    fn serialize_items(&self, buf: &mut MDDoc) {
        for r in &self.rows {
            if r.iter().any(|w| w.do_serialize()) {
                buf.body.push('*' as u8);
                buf.body.push(' ' as u8);
                for w in r  {
                    w.serialize(buf);
                    buf.body.push(' ' as u8);
                }
                buf.body.push('\n' as u8);
            }
        }
    }
    fn measure_items(&self, _: &DrawCtx) -> Point {
        self.size
    }
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point {
        if let Some(max_n_col) = self.rows.iter().max_by_key(|r| r.len()).map(|r| r.len()) {
            let mut max_col_widths: Vec<f32> = vec![0.; max_n_col];
            let mut row_heights: Vec<f32> = vec![0.; self.rows.len()];
            for (r, row) in self.rows.iter_mut().enumerate() {
                for (c, widget) in row.iter_mut().enumerate() {
                    let m = widget.remeasure(ctx); 
                    self.widget_rects[r][c] = Rect { c1: Point::origin(), c2: m };
                    max_col_widths[c] = max_col_widths[c].max(m.x);
                    row_heights[r] = row_heights[r].max(m.y);
                }
            }
            let mut row_offset = 0.;
            for (r, row) in self.widget_rects.iter_mut().enumerate() {
                let mut col_offset = 0.;
                for (c, rect) in row.iter_mut().enumerate() {
                    rect.set_offset(&Point::new(col_offset, row_offset));
                    col_offset += max_col_widths[c] + self.spacing.x;
                }
                row_offset += row_heights[r] + self.spacing.y;
                self.size.x = self.size.x.max(col_offset);
            }
            self.size.y = self.size.y.max(row_offset);
            return self.size
        }
        Point::origin()
    }
    fn select_first(&mut self) -> Option<&mut SelectionItem> {
        self.select_list.head.as_mut()
    }
}

pub fn new_label<T: Into<String>>(text: T) -> Box<dyn Widget> {
    Box::new(Label::new(text, None, None, None, TextParams::new()))
}

pub struct DateWidget {
    wl: WidgetList
}

impl DateWidget {
    pub fn new(ctx: &DrawCtx) -> Self {
        let mut wlb = WidgetList::new(Orientation::Horizontal, 10).builder(ctx);
        let local = chrono::Local::now();
        let (day, mon, year) = (local.day(), local.month(), local.year());
        wlb += new_textbox(2, &format!("{}", mon), ctx);
        wlb += new_textbox(2, &format!("{}", day), ctx);
        wlb += new_textbox(4, &format!("{}", year), ctx);
        DateWidget { wl: wlb.get() }
    }
}

impl Widget for DateWidget {
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        self.wl.draw(offset, ctx);
    }
    fn measure(&self, ctx: &DrawCtx) -> Point {
        self.wl.measure(ctx)
    }
    fn hover(&mut self, off: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.wl.hover(off, ctx)
    }
    fn click(&mut self, off: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.wl.click(off, ctx)
    }
    fn deselect(&mut self) -> Option<WidgetResponse> { 
        self.wl.deselect()
    }
    fn selection(&mut self) -> Option<&mut SelectionItem> {
        self.wl.selection()
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.wl.remeasure(ctx)
    }
    fn serialize(&self, buf: &mut MDDoc) {
        self.wl.get_widget(0).unwrap().serialize(buf);
        buf.body.push('/' as u8);
        self.wl.get_widget(1).unwrap().serialize(buf);
        buf.body.push('/' as u8);
        self.wl.get_widget(2).unwrap().serialize(buf);
        let local = chrono::Local::now();
        buf.body.push(' ' as u8);
        buf.title.date = local.to_rfc3339();
    }
}

pub struct Label {
    text: String,
    bg_color: Option<glm::Vec4>,
    hover_color: Option<glm::Vec4>,
    is_hover: bool,
    min_width: Option<f32>,
    text_params: TextParams,
}

impl Label {
    pub fn new<T: Into<String>>(text: T, bg_color: Option<glm::Vec4>, hover_color: Option<glm::Vec4>,
               min_width: Option<f32>, text_params: TextParams) -> Self {
        Label { text: text.into(), bg_color, hover_color, min_width, is_hover: false, text_params }
    }
}

impl Widget for Label {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        let mut m = ctx.render_text.measure(&self.text, self.text_params.scale);
        if let Some(min_width) = self.min_width {
            m.x = m.x.max(min_width);
        }
        m
    }
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        let mut m = ctx.render_text.measure(&self.text, self.text_params.scale);
        if let Some(min_width) = self.min_width { 
            m.x = m.x.max(min_width);
        }
        let r = Rect { c1: *offset, c2: *offset + m };
        let mut bg_color = self.bg_color;
        if self.is_hover && self.hover_color.is_some() {
            bg_color = self.hover_color;
        }
        if let Some(bg_color) = bg_color {
            ctx.draw_rect(r.clone(), bg_color, true, Radians(0.));
        }
        let rr = RotateRect::from_rect(r, Radians(0.));
        ctx.render_text.draw(&self.text, &self.text_params, &rr, ctx);
    }
    fn hover(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> {
        if self.hover_color.is_some() && !self.is_hover {
            self.is_hover = true;
            Some(just_status(WidgetStatus::REDRAW))
        }
        else { Some(just_status(WidgetStatus::FINE)) }
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        if self.is_hover {
            self.is_hover = false;
            Some(just_status(WidgetStatus::REDRAW))
        } else { None }
    }
    fn serialize(&self, buf: &mut MDDoc) {
        buf.body.extend_from_slice(self.text.as_bytes())
    }
}

pub fn new_textbox(num_chars: usize, default_text: &str, ctx: &DrawCtx) -> Box<dyn Widget> {
    Box::new(TextBox::new(default_text, ctx.render_text.measure(&String::from_utf8(
        "A".as_bytes().iter().cycle().take(num_chars).map(|c| *c).collect()).unwrap(), 1.0)))
}

pub fn new_dropdown<'a, T: Into<String> + AsRef<str> + 'static>(values: Vec<T>, selected: usize, ctx: &DrawCtx) -> Box<dyn Widget> {
    Box::new(DropDown::new(values, selected, ctx)) 
}

pub fn new_h_list(widgets: Vec<Box<dyn Widget>>, spacing: u32, ctx: &DrawCtx) -> Box<dyn Widget> {
    let mut wl = WidgetList::new(Orientation::Horizontal, spacing);
    for w in widgets {
        wl.add(w, ctx);
    }
    Box::new(wl)
}

pub fn new_v_list(widgets: Vec<Box<dyn Widget>>, spacing: u32, ctx: &DrawCtx) -> Box<dyn Widget> {
    let mut wl = WidgetList::new(Orientation::Vertical, spacing);
    for w in widgets {
        wl.add(w, ctx);
    }
    Box::new(wl)
}

#[derive(Debug)]
pub struct DropDownSelect {
    selected: usize,
    is_focus: bool,
    open: bool,
    max_value: usize
}

impl DropDownSelect {
    fn new(selected: usize, is_focus: bool, max_value: usize) -> Self {
        DropDownSelect { selected, is_focus, max_value, open: false }
    }
}

impl SelectionT for DropDownSelect {
    fn on_select(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> { 
        self.is_focus = true;
        Some(just_status(WidgetStatus::REDRAW))
    }
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> {
        self.is_focus = false;
        if !self.open {
            Some(just_status(WidgetStatus::REDRAW))
        }
        else {
            self.open = false;
            Some(just_status(WidgetStatus::REMEASURE))
        }
    }
    fn handle_key_down(&mut self, kc: &Keycode, _: &EventCtx) -> Option<WidgetResponse> {
        match *kc {
            Keycode::Down => { 
                if self.selected < self.max_value {
                    self.selected += 1;
                    Some(just_status(WidgetStatus::REDRAW))
                }
                else { None }
            }
            Keycode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Some(just_status(WidgetStatus::REDRAW))
                }
                else { None }
            }
            _ => None
        }
    }
    fn log(&self) {
        println!("{:?}", self);
    }
}

pub struct DropDown {
    selected: usize,
    hover_idx: usize,
    values_list: WidgetList,
    open: bool,
    dd_select: Rc<RefCell<DropDownSelect>>,
    selection: SelectionItem
}

impl DropDown {
    pub fn new<T: Into<String> + AsRef<str>>(values: Vec<T>, selected: usize, ctx: &DrawCtx) -> Self {
        let mut values_list = WidgetList::new(Orientation::Vertical, 0);
        let n_vals = values.len();
        let char_size = ctx.render_text.char_size('a', 1.0);
        let white = rgb_to_f32(255, 255, 255);
        let lb = rgb_to_f32(168, 238, 240);
        let mut max_width: f32 = 0.;
        for v in values.iter() {
            max_width = max_width.max(ctx.render_text.measure(v.as_ref(), 1.0).x);
        }
        max_width += char_size.x;
        for v in values.into_iter() {
            values_list.add(Box::new(Label::new(v, Some(white), Some(lb), Some(max_width), TextParams::new())), ctx);
        }
        let dd_select = Rc::new(RefCell::new(DropDownSelect::new(selected, false, n_vals - 1)));
        let selection = SelectionItem::new(dd_select.clone());
        DropDown { values_list, selected, hover_idx: 0, open: false, dd_select, selection }
    }
    fn draw_triangle(&self, off: &Point, ctx: &DrawCtx) {
        let char_size = ctx.render_text.char_size('a', 1.0);
        let blue = glm::vec4(0.,0.,1., 1.);
        let tri_center = Point::new(
            off.x + self.values_list.size.x - char_size.x / 2.,
            off.y + char_size.y / 2.);
        ctx.draw_iso_tri(tri_center, char_size.x, char_size.y, blue, true, Radians(std::f32::consts::PI));
    }
}

impl Widget for DropDown {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        if !self.dd_select.borrow().open {
            self.values_list.get_widget(0).as_ref().unwrap().measure(ctx)
        }
        else {
            self.values_list.measure(ctx)
        }
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        if !self.open {
            self.values_list.get_widget_mut(0).as_mut().unwrap().remeasure(ctx)
        }
        else {
            self.values_list.remeasure(ctx)
        }
    }
    fn draw(&self, off: &Point, ctx: &DrawCtx) {
        let open = self.dd_select.borrow().open;
        let selected = self.dd_select.borrow().selected;
        if !open {
            self.values_list.get_widget(selected).map(|w| w.draw(off, ctx));
        }
        else {
            self.values_list.draw(off, ctx);
        }
        self.draw_triangle(off, ctx);
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let open = self.dd_select.borrow().open;
        if open {
            self.selected = self.values_list.get_idx(off_pt, ctx.draw_ctx).unwrap_or(0);
            self.values_list.get_widget_mut(self.hover_idx).and_then(|w| w.deselect());
        }
        else {
            self.values_list.get_widget_mut(self.hover_idx).and_then(|w| w.hover(off_pt, ctx));
        }
        self.dd_select.borrow_mut().open = !open;
        let select = self.selection.clone();
        Some((WidgetStatus::REMEASURE,
            Rc::new(move |app: &mut AppState| {
                app.set_select(Some(Box::new(select.clone())));
            })))
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let open = self.dd_select.borrow().open;
        if open {
            let hover_idx = self.values_list.get_idx(off_pt, ctx.draw_ctx).unwrap_or(0);
            if hover_idx != self.hover_idx {
                self.values_list.get_widget_mut(self.hover_idx).map(|w| w.deselect());
                self.hover_idx = hover_idx;
            }
            self.values_list.get_widget_mut(self.hover_idx).and_then(|w| w.hover(off_pt, ctx))
        }
        else {
            None
        }
    }
    fn serialize(&self, buf: &mut MDDoc) {
        self.values_list.get_widget(self.selected).map(|w| w.serialize(buf));
    }
    fn selection(&mut self) -> Option<&mut SelectionItem> {
        Some(&mut self.selection)
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        let r = self.values_list.deselect();
        if self.open {
            self.open = false;
            Some(just_status(WidgetStatus::REMEASURE))
        }
        else { r }
    }
}

pub struct Button {
    pub onclick: WidgetResponse,
    border_rect: BorderRect,
    rect: [Rect; 1],
    label: [Box<dyn Widget>; 1]
}

impl Button {
    pub fn new(border: Border, fill_color: glm::Vec4, onclick: WidgetResponse) -> Self 
    {
        let size = Point::origin();
        let rect = Rect::empty();
        let border_rect = BorderRect::new(size, fill_color, border);
        Button { onclick, border_rect, label: [new_label("")], rect: [rect] }
    }
}

impl WidgetIterT for Button {
    type Child = Label; 
    fn add(&mut self, item: Self::Child, ctx: &DrawCtx) {
        let size = ctx.render_text.measure(&item.text, item.text_params.scale);
        self.border_rect.size = size;
        self.label[0] = Box::new(item);
        let off = self.border_rect.border.width;
        self.rect[0] = Rect { c1: off, c2: off + size };
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>)> + 'a> {
        Box::new(self.label.iter_mut())
    }
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(self.label.iter().zip(self.rect.iter()))
    }
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        Box::new(self.label.iter_mut().zip(self.rect.iter_mut()))
    }
    fn measure_items(&self, _: &DrawCtx) -> Point {
       self.border_rect.size + self.border_rect.border.width * Point::new(2., 2.)
    }
    fn hover_self(&mut self, _: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        *ctx.cursor = SystemCursor::Hand;
        Some(just_status(WidgetStatus::FINE))
    }
    fn draw_self(&self, offset: &Point, ctx: &DrawCtx) {
        self.border_rect.draw(*offset, ctx);
    }
    fn click_self(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> {
        Some((self.onclick.0, Rc::clone(&self.onclick.1)))
    }
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point {
        self.measure_items(ctx)
    }
    fn serialize_items(&self, _: &mut MDDoc) { }
}

pub trait SerializeT { 
    fn do_serialize() -> bool { true }
    fn serialize(_: &Box<dyn Widget>, buf: &mut MDDoc);
}

pub struct StrategySerializer { }
pub struct SymbolSerializer { }
pub struct PortfolioSerializer { }
pub struct SkipSerializer { }

impl SerializeT for StrategySerializer {
    fn serialize(w: &Box<dyn Widget>, buf: &mut MDDoc) {
        let mut tmp = MDDoc::empty();
        w.serialize(&mut tmp);
        buf.title.strategy = String::from_utf8(tmp.body).unwrap();
        w.serialize(buf);
    }
}

impl SerializeT for SymbolSerializer {
    fn serialize(w: &Box<dyn Widget>, buf: &mut MDDoc) {
        let mut tmp = MDDoc::empty();
        w.serialize(&mut tmp);
        buf.title.symbol = String::from_utf8(tmp.body).unwrap();
        w.serialize(buf);
    }
}

impl SerializeT for PortfolioSerializer {
    fn serialize(w: &Box<dyn Widget>, buf: &mut MDDoc) {
        let mut tmp = MDDoc::empty();
        w.serialize(&mut tmp);
        buf.portfolio = String::from_utf8(tmp.body).unwrap();
        w.serialize(buf);
    }
}

impl SerializeT for SkipSerializer {
    fn serialize(_: &Box<dyn Widget>, _: &mut MDDoc) { }
    fn do_serialize() -> bool { false }
}

pub struct SerializeWidget<T: SerializeT> {
    serializer: std::marker::PhantomData<T>,
    widget: Box<dyn Widget>
}

pub fn new_serialize<T: SerializeT + 'static>(widget: Box<dyn Widget>) -> Box<dyn Widget> {
    Box::new(SerializeWidget::<T>{ serializer: std::marker::PhantomData::<T>, widget })
}

impl<T: SerializeT> Widget for SerializeWidget<T> {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        self.widget.measure(ctx)
    }
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        self.widget.draw(offset, ctx);
    }
    fn click(&mut self, off: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.widget.click(off, ctx)
    }
    fn hover(&mut self, off: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.widget.hover(off, ctx)
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.widget.remeasure(ctx)
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        self.widget.deselect()
    }
    fn do_serialize(&self) -> bool {
        T::do_serialize()
    }
    fn serialize(&self, buf: &mut MDDoc) {
        T::serialize(&self.widget, buf);
    }
}

pub trait SelectionT {
    fn on_select(&mut self, ctx: &mut EventCtx) -> Option<WidgetResponse>;
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> { 
        None 
    }
    fn handle_key_down(&mut self, _: &Keycode, _: &EventCtx) -> Option<WidgetResponse> {
        None
    }
    fn log(&self);
}

#[derive(Clone)]
pub struct SelectionItem {
    pub select: Rc<RefCell<dyn SelectionT>>,
    pub prev: Option<Box<SelectionItem>>,
    pub next: Option<Box<SelectionItem>>
}

impl SelectionItem {
    pub fn new(select: Rc<RefCell<dyn SelectionT>>) -> Self {
        SelectionItem { select, prev: None, next: None }
    }
}

pub struct SelectionLinkedList {
    pub head: Option<SelectionItem>,
    pub tail: Option<SelectionItem>
}

impl SelectionLinkedList {
    fn new() -> Self {
        SelectionLinkedList { head: None, tail: None }
    }
    fn add(&mut self, select: &mut SelectionItem) {
        println!("Adding: ");
        select.select.borrow().log();
        if self.head.is_none() {
            self.head = Some(select.clone());
        }
        if let Some(ref mut tail) = self.tail {
            tail.next = Some(Box::new(select.clone()));
            select.prev = Some(Box::new(tail.clone()))
        }
        self.tail = Some(select.clone());
        println!("New list: ");
        self.print();
    }
    #[allow(dead_code)]
    pub fn print(&self) {
        let mut cur = self.head.as_ref().map(|h| Box::new(h.clone()));
        while let Some(ref s) = cur {
            s.select.borrow().log();
            cur = cur.unwrap().next;
        } 
    }
}

struct SelectTree<'a> {
    root: Box<SelectNode<'a>>
}

impl<'a> SelectTree<'a> {
    fn iter(&'a self) -> SelectTreeIt<'a> {
        SelectTreeIt { stack: vec![(&self.root, 0)] }
    }
}

struct SelectNode<'a> {
    select: &'a dyn SelectionT,
    children: Vec<Box<SelectNode<'a>>>
}

impl<'a> SelectNode<'a> {
    fn new(select: &'a dyn SelectionT, children: Vec<Box<SelectNode<'a>>>) -> Self {
        SelectNode {
            select,
            children
        }
    }
    fn add(&mut self, s: SelectNode<'a>) {
        self.children.push(Box::new(s));
    }
}

struct SelectTreeIt<'a> {
    stack: Vec<(&'a Box<SelectNode<'a>>, usize)>
}

impl<'a> Iterator for SelectTreeIt<'a> {
    type Item = &'a Box<SelectNode<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((ref cur, ref mut child)) = self.stack.last_mut() {
            if *child == 0 {
                *child += 1;
                return Some(cur);
            }
            else if *child <= cur.children.len() {
                let c = (&cur.children[*child - 1], 0);
                *child += 1;
                self.stack.push(c);
            }
            else {
                self.stack.pop();
            }
        }
        None
    }
}

