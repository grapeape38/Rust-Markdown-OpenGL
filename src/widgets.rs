extern crate nalgebra_glm;
extern crate bitflags;

use crate::interface::{CallbackFn, EventCtx, AppState};
use crate::render_text::{TextParams};
use crate::textedit::{TextBox};
use crate::primitives::{DrawCtx, Point, Rect, RotateRect, Radians, Border, BorderRect, InBounds, rgb_to_f32};
use sdl2::mouse::SystemCursor;
use nalgebra_glm as glm;
use bitflags::bitflags;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use chrono::Datelike;

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
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.measure(ctx)
    }
    fn serialize(&self, _: &mut Vec<u8>) {

    }
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

pub struct WidgetList {
    pub orientation: Orientation,
    pub spacing: u32,
    pub size: Point,
    widgets: Vec<Box<dyn Widget>>,
    widget_rects: Vec<Rect>,
    needs_draw: RefCell<Vec<bool>>
}

impl WidgetList {
    pub fn new(orientation: Orientation, spacing: u32) -> Self {
        WidgetList { orientation, spacing, widgets: Vec::new(), widget_rects: Vec::new(), needs_draw: RefCell::new(Vec::new()), 
                     size: Point::origin() }
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
    fn add(&mut self, item: Self::Child, ctx: &DrawCtx) {
        let m = item.measure(ctx);
        let spacing = if self.widgets.is_empty() { 0. } else { self.spacing as f32 };
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
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>)> + 'a> {
        Box::new(self.widgets.iter_mut())
    }
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(self.widgets.iter().zip(self.widget_rects.iter()))
    }
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        Box::new(self.widgets.iter_mut().zip(self.widget_rects.iter_mut()))
    }
    fn serialize_items(&self, buf: &mut Vec<u8>) {
        match self.orientation {
            Orientation::Vertical => {
                for w in &self.widgets {
                    buf.push('*' as u8);
                    buf.push(' ' as u8);
                    w.serialize(buf);
                    buf.push('\n' as u8);
                }
            }
            Orientation::Horizontal => {
                for w in &self.widgets {
                    w.serialize(buf);
                    buf.push(' ' as u8);
                }
            }
        }
    }
}

/*pub type WidgetChild<'a> = &'a Box<dyn Widget>; 
pub type WidgetIter<'a> = Box<dyn Iterator<Item=WidgetChild<'a>> + 'a>;
pub type WidgetIntoIter<'a> = Box<dyn IntoIterator<IntoIter=WidgetIter<'a>, Item=WidgetChild<'a>>>;

pub type WidgetChildMut<'a> = &'a mut Box<dyn Widget>; 
pub type WidgetIterMut<'a> = Box<dyn Iterator<Item=WidgetChildMut<'a>> + 'a>;
pub type WidgetIntoIterMut<'a> = Box<dyn IntoIterator<IntoIter=WidgetIterMut<'a>, Item=WidgetChildMut<'a>>>;

pub type WidgetChildRect<'a> = (WidgetChild<'a>, &'a Rect);
pub type WidgetChildRectIter<'a> = Box<dyn Iterator<Item=WidgetChildRect<'a>> + 'a>;
pub type WidgetChildRectIntoIter<'a> = Box<dyn IntoIterator<IntoIter=WidgetChildRectIter<'a>, Item=WidgetChildRect<'a>>>;

pub type WidgetChildRectMut<'a> = (WidgetChildMut<'a>, &'a mut Rect);
pub type WidgetChildRectIterMut<'a> = Box<dyn Iterator<Item=WidgetChildRect<'a>> + 'a>;
pub type WidgetChildRectIntoIterMut<'a> = Box<dyn IntoIterator<IntoIter=WidgetChildRectIter<'a>, Item=WidgetChildRect<'a>>>;*/

pub trait WidgetIterT {
    type Child;
    fn add(&mut self, item: Self::Child, ctx: &DrawCtx); 
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>)> + 'a>;
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a>;
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a>;
    fn measure_items(&self, ctx: &DrawCtx) -> Point;
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point;
    fn serialize_items(&self, buf: &mut Vec<u8>);
    fn click_self(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> { None }
    fn hover_self(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> { None }
    fn draw_self(&self, _: &Point, _: &DrawCtx) { }
    fn builder<'a>(self, ctx: &'a DrawCtx) -> WidgetBuilder<'a, Self> where Self: std::marker::Sized {
        WidgetBuilder::new(self, ctx)
    }
    fn handle_response(&mut self, resp: Option<(usize, WidgetResponse)>) -> Option<WidgetResponse> {
        if let Some(_) = resp {
            return Some(resp.unwrap().1)
        }
        None
    }
}

pub struct WidgetBuilder<'a, T: WidgetIterT> {
    w: T,
    ctx: &'a DrawCtx
}

impl<'a, T: WidgetIterT> WidgetBuilder<'a, T> {
    pub fn new(w: T, ctx: &'a DrawCtx) -> Self {
        Self { w, ctx }
    }
    pub fn get(self) -> T {
        self.w
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
            match resp {
                Some(r) => { 
                    match w.deselect() {
                        Some(ref r2) => Some(combine_response(&r, r2)),
                        None => Some(r)
                    }
                }
                None => { w.deselect() }
            }
        })
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let mut resp = self.click_self(off_pt, ctx).map(|r| (0, r));
        for (i, (w, r)) in self.widgets_plus_rects_mut().enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = w.click(&(*off_pt - r.c1), ctx).map(|r| (i, r));
                break;
            }
        }
        if let Some((i, _)) = resp {
            for (j, w) in self.widgets_mut().enumerate() {
                if i != j {
                    if let Some(wr2) = w.deselect() {
                        resp = Some((i, combine_response(&resp.unwrap().1, &wr2)));
                    }
                }
            }
        }
        self.handle_response(resp)
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let mut resp = self.hover_self(off_pt, ctx).map(|r| (0, r));
        for (i, (w, rect)) in self.widgets_plus_rects_mut().enumerate() {
            if rect.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = w.hover(&(*off_pt - rect.c1), ctx).map(|r| (i, r));
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
    fn serialize(&self, buf: &mut Vec<u8>) {
        self.serialize_items(buf)
    }
}

pub struct WidgetGrid {
    rows: Vec<Vec<Box<dyn Widget>>>,
    widget_rects: Vec<Vec<Rect>>,
    spacing: Point,
    size: Point
}

impl WidgetGrid {
    pub fn new(spacing: Point) -> Self {
        WidgetGrid { rows: Vec::new(), widget_rects: Vec::new(), spacing, size: Point::origin() }
    }
}

impl WidgetIterT for WidgetGrid {
    type Child = Vec<Box<dyn Widget>>;
    fn add(&mut self, new_row: Self::Child, ctx: &DrawCtx) {
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
        for (i, w) in new_row.iter().enumerate() {
            let m = w.measure(ctx);
            max_col_widths[i] = max_col_widths[i].max(m.x);
            let offset = Point::new(0., self.size.y);
            let new_rect = Rect { c1: offset, c2: offset + m };
            new_rects.push(new_rect);
            max_height = max_height.max(m.y);
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
    fn serialize_items(&self, buf: &mut Vec<u8>) {
        for r in &self.rows {
            buf.push('*' as u8);
            buf.push(' ' as u8);
            for w in r  {
                w.serialize(buf);
                buf.push(' ' as u8);
            }
            buf.push('\n' as u8);
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
}

pub fn new_label<T: Into<String>>(text: T) -> Box<dyn Widget> {
    Box::new(Label::new(text, None, None, None, TextParams::new()))
}

/*pub fn label_pair<T: Into<String>>(text: T, second: Box<dyn Widget>, ctx: &DrawCtx) -> Box<dyn Widget> {
    let wlb = WidgetListBuilder::new(Orientation::Horizontal, 10, ctx)
        + new_label(text)
        + second;
    wlb.get_widget()
}*/

pub struct DateWidget {
    wl: WidgetList
}

impl DateWidget {
    pub fn new(ctx: &DrawCtx) -> Self {
        let mut wlb = WidgetList::new(Orientation::Horizontal, 10).builder(ctx);
        let local = chrono::Local::now();
        let (day, mon, year) = (local.day(), local.month(), local.year());
        let n_days: [i8; 12] = [31, 28, 31, 30, 31, 30, 31, 30, 30, 31, 30, 31];
        let n_days_mon = n_days[(mon - 1) as usize];
        let day_strs: Vec<String> = (0..n_days_mon).map(|d: i8| format!("{}", d)).collect();
        let mon_strs: Vec<String> = (1..13).map(|m| format!("{}", m)).collect();
        let year_strs: Vec<String> = (year-9..year+1).map(|y| format!("{}", y)).collect();
        let n_years = year_strs.len();
        wlb += Box::new(DropDown::new(mon_strs, (mon - 1) as usize, ctx));
        wlb += Box::new(DropDown::new(day_strs, (day - 1) as usize, ctx));
        wlb += Box::new(DropDown::new(year_strs, n_years - 1, ctx));
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
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.wl.remeasure(ctx)
    }
    fn serialize(&self, buf: &mut Vec<u8>) {
        self.wl.get_widget(0).unwrap().serialize(buf);
        buf.push('/' as u8);
        self.wl.get_widget(1).unwrap().serialize(buf);
        buf.push('/' as u8);
        self.wl.get_widget(2).unwrap().serialize(buf);
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
    fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.text.as_bytes())
    }
}

pub fn new_textbox(num_chars: usize, ctx: &DrawCtx) -> Box<dyn Widget> {
    Box::new(TextBox::new(ctx.render_text.measure(&String::from_utf8(
        "A".as_bytes().iter().cycle().take(num_chars).map(|c| *c).collect()).unwrap(), 1.0)))
}

pub fn new_dropdown<'a, T: Into<String> + AsRef<str> + 'static>(values: Vec<T>, selected: usize, ctx: &DrawCtx) -> Box<dyn Widget> {
    Box::new(DropDown::new(values, selected, ctx)) 
}

pub struct DropDown {
    selected: usize,
    hover_idx: usize,
    values_list: WidgetList,
    open: bool,
}

impl DropDown {
    pub fn new<T: Into<String> + AsRef<str>>(values: Vec<T>, selected: usize, ctx: &DrawCtx) -> Self {
        let mut values_list = WidgetList::new(Orientation::Vertical, 0);
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
        DropDown { values_list, selected, hover_idx: 0, open: false }
    }
    fn draw_triangle(&self, off: &Point, ctx: &DrawCtx) {
        let char_size = ctx.render_text.char_size('a', 1.0);
        //let height = self.values_list.get_widget(0).as_ref().unwrap().measure(ctx).y;
        let blue = glm::vec4(0.,0.,1., 1.);
        let tri_center = Point::new(
            off.x + self.values_list.size.x - char_size.x / 2.,
            off.y + char_size.y / 2.);
        ctx.draw_iso_tri(tri_center, char_size.x, char_size.y, blue, true, Radians(std::f32::consts::PI));
    }
}

impl Widget for DropDown {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        if !self.open {
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
        if !self.open {
            self.values_list.get_widget(self.selected).map(|w| w.draw(off, ctx));
        }
        else {
            self.values_list.draw(off, ctx);
        }
        self.draw_triangle(off, ctx);
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        if self.open {
            self.selected = self.values_list.get_idx(off_pt, ctx.draw_ctx).unwrap_or(0);
            self.values_list.get_widget_mut(self.hover_idx).and_then(|w| w.deselect());
        }
        else {
            self.values_list.get_widget_mut(self.hover_idx).and_then(|w| w.hover(off_pt, ctx));
        }
        self.open = !self.open;
        Some(just_status(WidgetStatus::REMEASURE))
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        if self.open {
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
    fn serialize(&self, buf: &mut Vec<u8>) {
        self.values_list.get_widget(self.selected).map(|w| w.serialize(buf));
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
    text: &'static str,
    text_params: TextParams,
    pub onclick: WidgetResponse,
    border_rect: BorderRect,
    label: Option<Box<dyn Widget>>,
}

impl Button {
    pub fn new(text: &'static str, text_params: TextParams, 
        border: Border, fill_color: glm::Vec4, onclick: WidgetResponse, ctx: &DrawCtx) -> Self 
    {
        let size = ctx.render_text.measure(text, text_params.scale);
        let border_rect = BorderRect::new(size, fill_color, border);
        Button { text, text_params, onclick, border_rect, label: None }
    }
}

/*impl WidgetIterT for Button {
    type Child = Label; 
    fn add(&mut self, item: Self::Child, ctx: &DrawCtx) {
        self.label = Box::new(item);
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>)> + 'a> {
        Box::new(vec![self.label].iter_mut())
        //self.deref_mut().widgets_mut()
    }
    fn widgets_plus_rects<'a>(&'a self) -> Box<dyn Iterator<Item=(&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        self.deref().widgets_plus_rects()
    }
    fn widgets_plus_rects_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        self.deref_mut().widgets_plus_rects_mut()
    }
    fn measure_items(&self, _: &DrawCtx) -> Point {
       self.border_rect.size + self.border_rect.border.width * Point::new(2., 2.)
    }
    fn hover_self(&mut self, pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        *ctx.cursor = SystemCursor::Hand;
        Some(just_status(WidgetStatus::FINE))
    }
    fn draw_self(&self, offset: &Point, ctx: &DrawCtx) {
        self.border_rect.draw(*offset, ctx);
        let bw = self.border_rect.border.width;
        let r = Rect {c1: *offset + bw, c2: *offset + bw + self.border_rect.size};
        let rr = RotateRect::from_rect(r, Radians(0.));
        ctx.render_text.draw(&self.text, &self.text_params, &rr, ctx);
    }
    fn click_self(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> {
        Some((self.onclick.0, Rc::clone(&self.onclick.1)))
    }
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point {
        self.deref_mut().remeasure_items(ctx)
    }
    fn serialize_items(&self, buf: &mut Vec<u8>) {
        self.deref().serialize_items(buf)
    }
}*/
impl Widget for Button {
    fn measure(&self, _: &DrawCtx) -> Point {
       self.border_rect.size + self.border_rect.border.width * Point::new(2., 2.)
    }
    fn hover(&mut self, _: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        *ctx.cursor = SystemCursor::Hand;
        Some(just_status(WidgetStatus::FINE))
    }
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        self.border_rect.draw(*offset, ctx);
        let bw = self.border_rect.border.width;
        let r = Rect {c1: *offset + bw, c2: *offset + bw + self.border_rect.size};
        let rr = RotateRect::from_rect(r, Radians(0.));
        ctx.render_text.draw(&self.text, &self.text_params, &rr, ctx);
    }
    fn click(&mut self, _: &Point, _: &mut EventCtx) -> Option<WidgetResponse> {
        Some((self.onclick.0, Rc::clone(&self.onclick.1)))
    }
}

/*pub trait SerializeT { 
    fn write_before(&self, _: &mut Vec<u8>) { }
    fn write_self(&self, _: &mut Vec<u8>) { }
    fn write_after(&self, _: &mut Vec<u8>) { }
    fn serialize(&self, buf: &mut Vec<u8>) {
        self.write_before(buf);
        self.write_self(buf);
        self.write_after(buf);
    }
}

pub struct SerializeWidgetIter {
    sct: SerializeChildrenTags
}

pub struct SerializeChildrenTags {
    before_child: &'static str,
    delimiter: &'static str,
    after_child: &'static str
}

impl SerializeChildrenTags {
    fn serialize<'a>(&self, mut children: Box<dyn Iterator<Item=Box<dyn SerializeT>> + 'a>, buf: &mut Vec<u8>) {
        if let Some(c) = children.nth(0) {
            buf.extend_from_slice(self.before_child.as_bytes());
            c.serialize(buf);
            buf.extend_from_slice(self.after_child.as_bytes());
        }
        for c in children {
            buf.extend_from_slice(self.delimiter.as_bytes());
            buf.extend_from_slice(self.before_child.as_bytes());
            c.serialize(buf);
            buf.extend_from_slice(self.after_child.as_bytes());
        }
    }
}

pub trait SerializeChildrenT {
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item=Box<dyn SerializeT>> + 'a>;
    fn write_before_child(&self, _: &mut Vec<u8>) { }
    fn write_after_child(&self, _: &mut Vec<u8>) { }
}*/