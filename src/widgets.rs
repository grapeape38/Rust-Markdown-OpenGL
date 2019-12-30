/*extern crate nalgebra_glm;
extern crate bitflags;*/

use crate::interface::{AppState, CallbackFn, EventCtx};
use crate::primitives::{
    rgb_to_f32, Border, BorderRect, DrawCtx, InBounds, Point, Radians, Rect, RotateRect,
};
use crate::render_text::TextParams;
use crate::textedit::TextBox;
use bitflags::bitflags;
use chrono::Datelike;
use nalgebra_glm as glm;
use sdl2::keyboard::Keycode;
use sdl2::mouse::SystemCursor;
use std::cell::RefCell;
use std::rc::Rc;
use std::any::Any;

pub struct MDTitle {
    pub symbol: String,
    pub strategy: String,
    pub date: String,
}

impl MDTitle {
    pub fn empty() -> Self {
        MDTitle {
            symbol: String::new(),
            strategy: String::new(),
            date: String::new(),
        }
    }
}

pub struct MDDoc {
    pub title: MDTitle,
    pub portfolio: String,
    pub body: Vec<u8>,
}

impl MDDoc {
    pub fn empty() -> Self {
        MDDoc {
            title: MDTitle::empty(),
            portfolio: String::new(),
            body: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

pub enum WidgetType {
    Widget,
    Container,
}

pub trait Widget {
    fn draw(&self, offset: &Point, ctx: &mut WidgetDrawCtx);
    fn measure(&self, _: &DrawCtx) -> Point {
        Point::origin()
    }
    fn hover(&mut self, _: &Point, _: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        None
    }
    fn click(&mut self, _: &Point, _: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        None
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        None
    }
    fn selection(&self) -> Option<Box<dyn SelectionT>> {
        None
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.measure(ctx)
    }
    fn do_serialize(&self) -> bool {
        true
    }
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
    fn serialize(&self, _: &mut MDDoc) {}
    fn children<'a>(&'a self) -> Option<WidgetsIter<'a>> {
        None
    }
    fn children_mut<'a>(&'a mut self) -> Option<WidgetsIterMut<'a>> {
        None
    }
    fn make_leaf(&mut self) {
        if let Some(children) = self.children_mut() {
            for c in children {
                c.make_leaf();
            }
        }
    }
    fn widget_type(&self) -> WidgetType {
        WidgetType::Widget
    }
}

pub type WidgetsIter<'a> = Box<dyn Iterator<Item = &'a Box<dyn Widget>> + 'a>;
pub type WidgetsIterMut<'a> = Box<dyn Iterator<Item = &'a mut Box<dyn Widget>> + 'a>;

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
    fn combine(self, other: Self) -> Self
    where
        Self: Sized;
}

impl CombineResponse for Option<WidgetResponse> {
    fn combine(self, other: Self) -> Self {
        match self {
            Some(r) => match other {
                Some(r2) => Some(combine_response(&r, &r2)),
                None => Some(r),
            },
            None => other,
        }
    }
}

pub fn combine_response(r1: &WidgetResponse, r2: &WidgetResponse) -> WidgetResponse {
    let cb1 = Rc::clone(&r1.1);
    let cb2 = Rc::clone(&r2.1);
    (
        r1.0 | r2.0,
        Rc::new(move |app: &mut AppState| {
            (cb1)(app);
            (cb2)(app);
        }),
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
    leaf: bool
}

impl WidgetList {
    pub fn new(orientation: Orientation, spacing: u32) -> Self {
        WidgetList {
            orientation,
            spacing,
            widgets: Vec::new(),
            widget_rects: Vec::new(),
            needs_draw: RefCell::new(Vec::new()),
            size: Point::origin(),
            leaf: false
        }
    }

    pub fn get_widget(&self, idx: usize) -> Option<&Box<dyn Widget>> {
        self.widgets.get(idx)
    }
    pub fn get_widget_mut(&mut self, idx: usize) -> Option<&mut Box<dyn Widget>> {
        self.widgets.get_mut(idx)
    }
    pub fn get_idx(&self, off_pt: &Point, ctx: &DrawCtx) -> Option<usize> {
        self.widget_rects
            .iter()
            .position(|r| r.in_bounds(off_pt, &ctx.viewport))
    }
}

impl WidgetIterT for WidgetList {
    type Child = Box<dyn Widget>;
    fn add(&mut self, item: Self::Child) {
        self.widgets.push(item);
        self.widget_rects.push(Rect::empty());
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
            self.widget_rects[i] = Rect {
                c1: off,
                c2: off + m,
            };
        }
        *self.needs_draw.borrow_mut() = vec![true; self.widgets.len()];
        self.size = size;
        size
    }
    fn widgets<'a>(&'a self) -> WidgetsIter<'a> {
        Box::new(self.widgets.iter())
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Box<dyn Widget>> + 'a> {
        Box::new(self.widgets.iter_mut())
    }
    fn widgets_plus_rects<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(self.widgets.iter().zip(self.widget_rects.iter()))
    }
    fn widgets_plus_rects_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
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
    fn is_leaf(&self) -> bool {
        self.leaf
    }
    fn make_leaf_self(&mut self) {
        self.leaf = true;
    }
}

pub trait WidgetIterT {
    type Child;
    fn add(&mut self, item: Self::Child/*, ctx: &mut WidgetDrawCtx*/);
    fn widgets<'a>(&'a self) -> WidgetsIter<'a>;
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Box<dyn Widget>> + 'a>;
    fn widgets_plus_rects<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a Box<dyn Widget>, &'a Rect)> + 'a>;
    fn widgets_plus_rects_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a>;
    fn measure_items(&self, ctx: &DrawCtx) -> Point;
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point;
    fn serialize_items(&self, buf: &mut MDDoc);
    fn deselect_self(&mut self) -> Option<WidgetResponse> {
        None
    }
    fn click_self(&mut self, _: &Point, _: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        None
    }
    fn hover_self(&mut self, _: &Point, _: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        None
    }
    fn is_leaf(&self) -> bool {
        false
    }
    fn make_leaf_self(&mut self) {}
    fn draw_self(&self, _: &Point, _: &mut WidgetDrawCtx) {}
    fn builder(self) -> WidgetBuilder<Self>
    where
        Self: std::marker::Sized + 'static,
    {
        WidgetBuilder::new(self)
    }
    fn handle_response(&mut self, resp: Option<WidgetResponse>) -> Option<WidgetResponse> {
        resp
    }
    fn widget_type_self(&self) -> WidgetType {
        WidgetType::Container
    }
}

pub struct WidgetBuilder<T: WidgetIterT> {
    w: T,
}

impl<T: WidgetIterT + 'static> WidgetBuilder<T> {
    pub fn new(w: T) -> Self {
        Self { w }
    }
    pub fn get(self) -> T {
        self.w
    }
    pub fn widget(self) -> Box<dyn Widget> {
        Box::new(self.w)
    }
}

impl<C, T: WidgetIterT<Child = C>> std::ops::Add<C> for WidgetBuilder<T> {
    type Output = Self;
    fn add(self, c: C) -> Self {
        let mut w = self.w;
        w.add(c);
        WidgetBuilder { w }
    }
}

impl<'a, C, T: WidgetIterT<Child = C>> std::ops::AddAssign<C> for WidgetBuilder<T> {
    fn add_assign(&mut self, c: C) {
        self.w.add(c);
    }
}

impl<T: WidgetIterT> Widget for T {
    fn draw(&self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        self.draw_self(offset, ctx);
        let leaf = self.is_leaf();
        for (w, r) in self.widgets_plus_rects() {
            w.draw(&(*offset + r.c1), ctx.next_widget_ctx(leaf));
        }
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        self.widgets_mut()
            .fold(None, |resp, w| resp.combine(w.deselect()))
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        let mut resp = self.click_self(off_pt, ctx);
        let w_idx = ctx.widget_idx;
        let leaf = self.is_leaf();
        for (c_idx, (w, r)) in self.widgets_plus_rects_mut().enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                //println!("Clicked in bounds! {:?} {:?}", w_idx, c_idx);
                resp = resp.combine(w.click(&(*off_pt - r.c1), ctx.child_ctx(w_idx, c_idx, leaf)));
                break;
            }
        }
        self.handle_response(resp)
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        let mut resp = self.hover_self(off_pt, ctx);
        let w_idx = ctx.widget_idx;
        let leaf = self.is_leaf();
        for (c_idx, (w, rect)) in self.widgets_plus_rects_mut().enumerate() {
            if rect.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = resp.combine(w.hover(&(*off_pt - rect.c1), ctx.child_ctx(w_idx, c_idx, leaf)));
                break;
            }
        }
        self.handle_response(resp)
    }
    fn make_leaf(&mut self) {
        self.make_leaf_self();
        for w in self.widgets_mut() {
            w.make_leaf();
        }
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
    fn children<'a>(&'a self) -> Option<WidgetsIter<'a>> {
        Some(self.widgets())
    }
    fn children_mut<'a>(&'a mut self) -> Option<WidgetsIterMut<'a>> {
        Some(self.widgets_mut())
    }
    fn widget_type(&self) -> WidgetType {
        self.widget_type_self()
    }
}

pub struct WidgetGrid {
    rows: Vec<Vec<Box<dyn Widget>>>,
    widget_rects: Vec<Vec<Rect>>,
    spacing: Point,
    size: Point,
}

impl WidgetGrid {
    pub fn new(spacing: Point) -> Self {
        WidgetGrid {
            rows: Vec::new(),
            widget_rects: Vec::new(),
            spacing,
            size: Point::origin(),
        }
    }
}

impl WidgetIterT for WidgetGrid {
    type Child = Vec<Box<dyn Widget>>;
    fn add(&mut self, new_row: Self::Child) {
        self.widget_rects.push(vec![Rect::empty(); new_row.len()]);
        self.rows.push(new_row);
    }
    fn widgets<'a>(&'a self) -> WidgetsIter<'a> {
        Box::new(self.rows.iter().flatten())
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Box<dyn Widget>> + 'a> {
        Box::new(self.rows.iter_mut().flatten())
    }
    fn widgets_plus_rects<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(
            self.rows
                .iter()
                .flatten()
                .zip(self.widget_rects.iter().flatten()),
        )
    }
    fn widgets_plus_rects_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        Box::new(
            self.rows
                .iter_mut()
                .flatten()
                .zip(self.widget_rects.iter_mut().flatten()),
        )
    }
    fn serialize_items(&self, buf: &mut MDDoc) {
        for r in &self.rows {
            if r.iter().any(|w| w.do_serialize()) {
                buf.body.push('*' as u8);
                buf.body.push(' ' as u8);
                for w in r {
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
                    self.widget_rects[r][c] = Rect {
                        c1: Point::origin(),
                        c2: m,
                    };
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
            return self.size;
        }
        Point::origin()
    }
}

pub fn new_label<T: Into<String>>(text: T) -> Box<dyn Widget> {
    Box::new(Label::new(text, None, None, None, TextParams::new()))
}

/*pub struct DateWidget {
    wl: WidgetList,
}

impl DateWidget {
    pub fn new() -> Self {
        let mut wlb = WidgetList::new(Orientation::Horizontal, 10).builder();
        let local = chrono::Local::now();
        let (day, mon, year) = (local.day(), local.month(), local.year());
        wlb += new_textbox(&format!("{}", mon), 2);
        wlb += new_textbox(&format!("{}", day), 4);
        wlb += new_textbox(&format!("{}", year), 4);
        DateWidget { wl: wlb.get() }
    }
}

impl WidgetWrapper for DateWidget {
    type Wrapped = WidgetList;
    fn wrapped(&self) -> &WidgetList {
        &self.wl
    }
    fn wrapped_mut(&mut self) -> &mut WidgetList {
        &mut self.wl
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
}*/

pub struct Label {
    text: String,
    bg_color: Option<glm::Vec4>,
    hover_color: Option<glm::Vec4>,
    is_hover: bool,
    min_width: Option<f32>,
    text_params: TextParams,
}

impl Label {
    pub fn new<T: Into<String>>(
        text: T,
        bg_color: Option<glm::Vec4>,
        hover_color: Option<glm::Vec4>,
        min_width: Option<f32>,
        text_params: TextParams,
    ) -> Self {
        Label {
            text: text.into(),
            bg_color,
            hover_color,
            min_width,
            is_hover: false,
            text_params,
        }
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
    fn draw(&self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        let dctx = &ctx.draw_ctx;
        let mut m = dctx.render_text.measure(&self.text, self.text_params.scale);
        if let Some(min_width) = self.min_width {
            m.x = m.x.max(min_width);
        }
        let r = Rect {
            c1: *offset,
            c2: *offset + m,
        };
        let mut bg_color = self.bg_color;
        if self.is_hover && self.hover_color.is_some() {
            bg_color = self.hover_color;
        }
        if let Some(bg_color) = bg_color {
            dctx.draw_rect(r.clone(), bg_color, true, Radians(0.));
        }
        let rr = RotateRect::from_rect(r, Radians(0.));
        dctx.render_text
            .draw(self.text.bytes(), &self.text_params, &rr, dctx);
    }
    fn hover(&mut self, _: &Point, _: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        if self.hover_color.is_some() && !self.is_hover {
            self.is_hover = true;
            Some(just_status(WidgetStatus::REDRAW))
        } else {
            Some(just_status(WidgetStatus::FINE))
        }
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        if self.is_hover {
            self.is_hover = false;
            Some(just_status(WidgetStatus::REDRAW))
        } else {
            None
        }
    }
    fn serialize(&self, buf: &mut MDDoc) {
        buf.body.extend_from_slice(self.text.as_bytes())
    }
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

pub fn new_textbox(default_text: &str, num_chars: usize) -> Box<dyn Widget> {
    Box::new(TextBox::new(
        default_text,
        num_chars
    ))
}

pub fn new_dropdown<'a, T: Into<String> + AsRef<str> + 'static>(
    values: Vec<T>,
    selected: usize,
) -> Box<dyn Widget> {
    Box::new(DropDown::new(values, selected))
}

pub fn new_h_list(widgets: Vec<Box<dyn Widget>>, spacing: u32) -> Box<dyn Widget> {
    let mut wl = WidgetList::new(Orientation::Horizontal, spacing);
    for w in widgets {
        wl.add(w);
    }
    Box::new(wl)
}

pub fn new_v_list(widgets: Vec<Box<dyn Widget>>, spacing: u32) -> Box<dyn Widget> {
    let mut wl = WidgetList::new(Orientation::Vertical, spacing);
    for w in widgets {
        wl.add(w);
    }
    Box::new(wl)
}

#[derive(Debug)]
pub struct DropDownSelect {
    selected: usize,
    //is_focus: bool,
    //open: bool,
    max_value: usize,
}

impl DropDownSelect {
    fn new(selected: usize, max_value: usize) -> Self {
        DropDownSelect {
            selected,
            max_value,
        }
    }
}

impl SelectionT for DropDownSelect {
    fn on_select(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> {
        //self.is_focus = true;
        Some(just_status(WidgetStatus::REDRAW))
    }
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> {
        //self.is_focus = false;
        //if !self.open {
        Some(just_status(WidgetStatus::REDRAW))
        //} else {
         //   self.open = false;
          //  Some(just_status(WidgetStatus::REMEASURE))
        //}
    }
    fn handle_key_down(&mut self, kc: &Keycode, _: &EventCtx) -> Option<WidgetResponse> {
        match *kc {
            Keycode::Down => {
                if self.selected < self.max_value {
                    self.selected += 1;
                    Some(just_status(WidgetStatus::REDRAW))
                } else {
                    None
                }
            }
            Keycode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Some(just_status(WidgetStatus::REDRAW))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn log(&self) {
        println!("{:?}", self);
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

pub struct DropDown {
    selected: usize,
    hover_idx: usize,
    values_list: WidgetList,
    open: bool,
}

impl DropDown {
    pub fn new<T: Into<String> + AsRef<str>>(
        values: Vec<T>,
        selected: usize,
    ) -> Self {
        let mut values_list = WidgetList::new(Orientation::Vertical, 0);
        let white = rgb_to_f32(255, 255, 255);
        let lb = rgb_to_f32(168, 238, 240);
        for v in values.into_iter() {
            values_list.add(
                Box::new(Label::new(
                    v,
                    Some(white),
                    Some(lb),
                    None,
                    TextParams::new(),
                )),
            );
        }
        values_list.make_leaf();
        //let selection = SelectionItem::new(dd_select.clone());
        DropDown {
            values_list,
            selected,
            hover_idx: 0,
            open: false,
            //dd_select,
            //selection,
        }
    }
    fn draw_triangle(&self, off: &Point, ctx: &mut WidgetDrawCtx) {
        let char_size = ctx.draw_ctx.render_text.char_size('a', 1.0);
        let blue = glm::vec4(0., 0., 1., 1.);
        let tri_center = Point::new(
            off.x + self.values_list.size.x - char_size.x / 2.,
            off.y + char_size.y / 2.,
        );
        ctx.draw_ctx.draw_iso_tri(
            tri_center,
            char_size.x,
            char_size.y,
            blue,
            true,
            Radians(std::f32::consts::PI),
        );
    }
}

impl Widget for DropDown {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        if !self.open {
            self.values_list
                .get_widget(0)
                .as_ref()
                .unwrap()
                .measure(ctx)
        } else {
            self.values_list.measure(ctx)
        }
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        let char_size = ctx.render_text.char_size('a', 1.0);
        let mut max_width: f32 = 0.;
        for v in self.values_list.widgets_mut() {
            let text = &downcast_widget::<Label>(v).text;
            max_width = max_width.max(ctx.render_text.measure(text, 1.0).x);
        }
        max_width += char_size.x;
        for v in self.values_list.widgets_mut() {
            downcast_widget_mut::<Label>(v).min_width = Some(max_width);
        }
        let full_size = self.values_list.remeasure(ctx);
        if !self.open {
            self.values_list
                .get_widget_mut(0)
                .as_mut()
                .unwrap()
                .remeasure(ctx)
        } else {
            full_size
        }
    }
    fn draw(&self, off: &Point, ctx: &mut WidgetDrawCtx) {
        if !self.open {
            self.values_list
                .get_widget(self.selected)
                .map(|w| w.draw(off, ctx));
        } else {
            self.values_list.draw(off, ctx);
        }
        self.draw_triangle(off, ctx);
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        if self.open {
            self.selected = self.values_list.get_idx(off_pt, ctx.draw_ctx).unwrap_or(0);
            self.values_list
                .get_widget_mut(self.hover_idx)
                .and_then(|w| w.deselect());
        } else {
            self.values_list
                .get_widget_mut(self.hover_idx)
                .and_then(|w| w.hover(off_pt, ctx));
        }
        self.open = !self.open;
        Some(just_status(WidgetStatus::REMEASURE))
        /*Some((
            WidgetStatus::REMEASURE,
            Rc::new(move |app: &mut AppState| {
                app.set_select(Some(Box::new(select.clone())));
            }),
        ))*/
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        if self.open {
            let hover_idx = self.values_list.get_idx(off_pt, ctx.draw_ctx).unwrap_or(0);
            if hover_idx != self.hover_idx {
                self.values_list
                    .get_widget_mut(self.hover_idx)
                    .map(|w| w.deselect());
                self.hover_idx = hover_idx;
            }
            self.values_list
                .get_widget_mut(self.hover_idx)
                .and_then(|w| w.hover(off_pt, ctx))
        } else {
            None
        }
    }
    fn serialize(&self, buf: &mut MDDoc) {
        self.values_list
            .get_widget(self.selected)
            .map(|w| w.serialize(buf));
    }
    /*fn selection(&mut self) -> Option<&mut SelectionItem> {
        Some(&mut self.selection)
    }*/
    fn deselect(&mut self) -> Option<WidgetResponse> {
        let r = self.values_list.deselect();
        if self.open {
            self.open = false;
            Some(just_status(WidgetStatus::REMEASURE))
        } else {
            r
        }
    }
    fn selection(&self) -> Option<Box<dyn SelectionT>> {
        Some(Box::new(
            DropDownSelect::new(self.selected, self.values_list.widgets.len())
        ))
    }
}

pub struct Button {
    pub onclick: WidgetResponse,
    border_rect: BorderRect,
    rect: [Rect; 1],
    label: [Box<dyn Widget>; 1],
}

impl Button {
    pub fn new(border: Border, fill_color: glm::Vec4, onclick: WidgetResponse) -> Self {
        let size = Point::origin();
        let rect = Rect::empty();
        let border_rect = BorderRect::new(size, fill_color, border);
        Button {
            onclick,
            border_rect,
            label: [new_label("")],
            rect: [rect],
        }
    }
}

fn downcast_widget<'a, W: Widget + 'static>(w: &'a Box<dyn Widget + 'static>) -> &'a W {
    w.as_any().unwrap().downcast_ref::<W>().unwrap()
}

fn downcast_widget_mut<'a, W: Widget + 'static>(w: &'a mut Box<dyn Widget + 'static>) -> &'a mut W {
    w.as_any_mut().unwrap().downcast_mut::<W>().unwrap()
}

impl WidgetIterT for Button {
    type Child = Label;
    fn add(&mut self, item: Self::Child) {
        self.label[0] = Box::new(item);
    }
    fn widgets<'a>(&'a self) -> WidgetsIter<'a> {
        Box::new(self.label.iter())
    }
    fn widgets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Box<dyn Widget>> + 'a> {
        Box::new(self.label.iter_mut())
    }
    fn widgets_plus_rects<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a Box<dyn Widget>, &'a Rect)> + 'a> {
        Box::new(self.label.iter().zip(self.rect.iter()))
    }
    fn widgets_plus_rects_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (&'a mut Box<dyn Widget>, &'a mut Rect)> + 'a> {
        Box::new(self.label.iter_mut().zip(self.rect.iter_mut()))
    }
    fn measure_items(&self, _: &DrawCtx) -> Point {
        self.border_rect.size + self.border_rect.border.width * Point::new(2., 2.)
    }
    fn hover_self(&mut self, _: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        *ctx.cursor = SystemCursor::Hand;
        Some(just_status(WidgetStatus::FINE))
    }
    fn draw_self(&self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        self.border_rect.draw(*offset, ctx.draw_ctx);
    }
    fn click_self(&mut self, _: &Point, _: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        Some((self.onclick.0, Rc::clone(&self.onclick.1)))
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn remeasure_items(&mut self, ctx: &DrawCtx) -> Point {
        let label = downcast_widget::<Label>(&self.label[0]);
        let size = ctx.render_text.measure(&label.text, label.text_params.scale);
        self.border_rect.size = size;
        let off = self.border_rect.border.width;
        self.rect[0] = Rect {
            c1: off,
            c2: off + size,
        };
        size
    }
    fn serialize_items(&self, _: &mut MDDoc) {}
}

pub trait SerializeT {
    fn do_serialize() -> bool {
        true
    }
    fn serialize(_: &Box<dyn Widget>, buf: &mut MDDoc);
}

pub struct StrategySerializer {}
pub struct SymbolSerializer {}
pub struct PortfolioSerializer {}
pub struct SkipSerializer {}

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
    fn serialize(_: &Box<dyn Widget>, _: &mut MDDoc) {}
    fn do_serialize() -> bool {
        false
    }
}

pub struct SerializeWidget<T: SerializeT> {
    serializer: std::marker::PhantomData<T>,
    widget: Box<dyn Widget>,
}

pub fn new_serialize<T: SerializeT + 'static>(widget: Box<dyn Widget>) -> Box<dyn Widget> {
    Box::new(SerializeWidget::<T> {
        serializer: std::marker::PhantomData::<T>,
        widget,
    })
}

impl<T: SerializeT> Widget for SerializeWidget<T> {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        self.widget.measure(ctx)
    }
    fn draw(&self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        self.widget.draw(offset, ctx);
    }
    fn click(&mut self, off: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        self.widget.click(off, ctx.next_widget_ctx(false))
    }
    fn hover(&mut self, off: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        self.widget.hover(off, ctx.next_widget_ctx(false))
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

/*pub struct WidgetArrayTree {
    selections: Vec<Option<Box <dyn SelectionT>>>,
    children: Vec<Option<usize>>
}*/
pub type SelectMap = Vec<Option<Box<dyn SelectionT>>>;
pub type ChildrenSizes = Vec<Vec<usize>>;

fn children_recurse<'a>(cur: &Box<dyn Widget + 'a>, pos: &mut usize, vec: &mut ChildrenSizes) -> usize {
    let mut size = 1;
    let idx = *pos;
    *pos += 1;
    vec.push(Vec::new());
    if let Some(children) = cur.children() {
        for c in children {
            let csize = children_recurse(c, pos, vec);
            vec[idx].push(csize);
            size += csize;
        }
    }
    size
}

fn children_sizes(root: &Box<dyn Widget>) -> ChildrenSizes {
    let mut vec: ChildrenSizes = Vec::new();
    let mut pos = 0;
    children_recurse(root, &mut pos, &mut vec);
    vec
}

pub trait SelectionT {
    fn on_select(&mut self, ctx: &mut EventCtx) -> Option<WidgetResponse>;
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> {
        None
    }
    fn handle_key_down(&mut self, _: &Keycode, _: &EventCtx) -> Option<WidgetResponse> {
        None
    }
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
    fn log(&self);
}

pub struct SelectionList {
    vec: Vec<Box<dyn SelectionT>>,
    widget_idx: Vec<Option<usize>>
}

impl SelectionList {
    fn recurse_build<'a>(cur: &Box<dyn Widget + 'a>, pos: &mut usize, 
        v: &mut Vec<Box<dyn SelectionT>>, widget_idx: &mut Vec<Option<usize>>) {
        if let Some(select) = cur.selection() {
            widget_idx.push(Some(v.len()));
            v.push(select);
        }
        else {
            widget_idx.push(None);
        }
        *pos += 1;
        if let Some(iter) = cur.children() {
            for w in iter {
                SelectionList::recurse_build(w, pos, v, widget_idx);
            }
        }
    }
    fn new(root: &Box<dyn Widget>) -> Self {
        let mut vec: Vec<Box<dyn SelectionT>> = Vec::new();
        let mut widget_idx: Vec<Option<usize>> = Vec::new();
        let mut pos = 0;
        SelectionList::recurse_build(root, &mut pos, &mut vec, &mut widget_idx);
        SelectionList {
            vec,
            widget_idx
        }
    }
}

pub struct SelectionState {
    list: SelectionList,
    cur_select: Option<usize>,
    child_sizes: ChildrenSizes
}

#[derive(Clone, Copy, Debug)]
pub struct WidgetIdx(pub usize);

impl SelectionState {
    pub fn new(root: &Box<dyn Widget>) -> Self {
        SelectionState {
            list: SelectionList::new(root),
            cur_select: None,
            child_sizes: children_sizes(root)
        }
    }
    pub fn is_select(&self) -> bool {
        self.cur_select.is_some()
    }
    pub fn set_select(&mut self, idx: Option<usize>, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let mut resp = self.cur_select
            .and_then(|idx| self.list.vec[idx].on_deselect(ctx));
        self.cur_select = idx;
        resp = resp.combine(
            self.cur_select
            .and_then(|idx| self.list.vec[idx].on_select(ctx))
        );
        resp
    }
    pub fn get(&self, idx: usize) -> &Box<dyn SelectionT> {
        &self.list.vec[idx]
    }
    pub fn get_mut(&mut self, idx: usize) -> &mut Box<dyn SelectionT> {
        &mut self.list.vec[idx]
    }
    pub fn select_next(&mut self, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        if let Some(ref mut idx) = self.cur_select {
            if *idx < self.list.vec.len() - 1 {
                *idx += 1;
                return self.on_select(ctx)
            }
        }
        None
    }
    pub fn get_select<'a, T: SelectionT + 'static>(&'a self, idx: usize) -> Option<&'a T> {
        self.list.vec[idx].as_any().and_then(|s| s.downcast_ref::<T>())
    }
    pub fn get_select_mut<'a, T: SelectionT + 'static>(&'a mut self, idx: usize) 
        -> Option<&'a mut T> {
        self.list.vec[idx].as_any_mut().and_then(|s| s.downcast_mut::<T>())
    }
    pub fn is_select_w(&self, widx: WidgetIdx) -> bool {
        self.list.widget_idx[widx.0] == self.cur_select
    }
    pub fn select_idx(&self, widx: WidgetIdx) -> Option<usize> {
        self.list.widget_idx[widx.0]
    }
    pub fn get_select_w<'a, T: SelectionT + 'static>(&'a self, widx: WidgetIdx) -> Option<&'a T> {
        //println!("{:?} {:?}", self.list.widget_idx[widx.0], widx.0);
        if let Some(idx) = self.list.widget_idx[widx.0] {
            self.list.vec[idx].as_any().and_then(|s| s.downcast_ref::<T>())
        } else {
            None
        }
    }
    pub fn get_select_w_mut<'a, T: SelectionT + 'static>(&'a mut self, widx: WidgetIdx)
        -> Option<&'a mut T> {
        if let Some(idx) = self.list.widget_idx[widx.0] {
            self.list.vec[idx].as_any_mut().and_then(|s| s.downcast_mut::<T>())
        } else {
            None
        }
    }
    pub fn print(&self) {
        println!("{:?}", self.list.widget_idx);
        println!("{:?}", self.child_sizes);
        for s in &self.list.vec {
            s.log();
        }
    }
    pub fn child_widget_idx(&self, parent_idx: WidgetIdx, child_pos: usize) -> WidgetIdx {
        let mut sz = 0;
        for i in 0..child_pos {
            sz += self.child_sizes[parent_idx.0][i];
        }
        WidgetIdx(parent_idx.0 + sz + 1)
    }
}

impl SelectionT for SelectionState {
    fn on_select(&mut self, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.cur_select.and_then(|idx| self.list.vec[idx].on_select(ctx))
    }
    fn on_deselect(&mut self, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.cur_select.and_then(|idx| self.list.vec[idx].on_deselect(ctx))
    }
    fn handle_key_down(&mut self, kc: &Keycode, ctx: &EventCtx) -> Option<WidgetResponse> {
        self.cur_select.and_then(|idx| self.list.vec[idx].handle_key_down(kc, ctx))
    }
    fn log(&self) { }
}

pub struct WidgetDrawCtx<'a> {
    pub draw_ctx: &'a DrawCtx,
    pub select_state: &'a SelectionState,
    pub widget_idx: WidgetIdx,
}

impl<'a> WidgetDrawCtx<'a> {
    pub fn new(draw_ctx: &'a DrawCtx, select_state: &'a SelectionState) -> Self {
        WidgetDrawCtx {
            draw_ctx, select_state, widget_idx: WidgetIdx(0)
        }
    }
    fn next_widget_ctx(&mut self, is_leaf: bool) -> &mut Self {
        if !is_leaf {
            self.widget_idx.0 += 1;
        }
        self
    }
    /*fn child_ctx(&mut self, w_idx: WidgetIdx, c_idx: usize) -> &mut Self {
        self.widget_idx = self.select_state.child_widget_idx(w_idx, c_idx);
        self
    }*/
    pub fn is_selected(&self) -> bool {
        self.select_state.is_select_w(self.widget_idx)
    }
    pub fn select_idx(&self) -> Option<usize> {
        self.select_state.select_idx(self.widget_idx)
    }
    pub fn get_select<T: SelectionT + 'static>(&'a self) -> Option<&'a T> {
        self.select_state.get_select_w(self.widget_idx)
    }
}

pub struct WidgetEventCtx<'a> {
    pub draw_ctx: &'a DrawCtx,
    pub cursor: &'a mut SystemCursor,
    pub select_state: &'a SelectionState,
    pub widget_idx: WidgetIdx,
}

impl<'a> WidgetEventCtx<'a> {
    pub fn new(draw_ctx: &'a DrawCtx, cursor: &'a mut SystemCursor, select_state: &'a SelectionState) -> Self {
        WidgetEventCtx {
            draw_ctx, select_state, cursor, widget_idx: WidgetIdx(0)
        }
    }
    fn next_widget_ctx(&mut self, is_leaf: bool) -> &mut Self {
        if !is_leaf { 
            self.widget_idx.0 += 1;
        }
        self
    }
    fn child_ctx(&mut self, parent_idx: WidgetIdx, child_pos: usize, is_leaf: bool) -> &mut Self {
        if !is_leaf {
            self.widget_idx = self.select_state.child_widget_idx(parent_idx, child_pos);
        }
        self
    }
    pub fn is_selected(&self) -> bool {
        self.select_state.is_select_w(self.widget_idx)
    }
    pub fn select_idx(&self) -> Option<usize> {
        self.select_state.select_idx(self.widget_idx)
    }
    pub fn get_select<T: SelectionT + 'static>(&'a self) -> Option<&'a T> {
        self.select_state.get_select_w(self.widget_idx)
    }
}

struct PushValue<'a, T: Copy> {
    val_ref: &'a mut T,
    saved_value: T
}

#[allow(dead_code)]
impl<'a, T: Copy> PushValue<'a, T> {
    pub fn new(val_ref: &'a mut T, new_value: T) -> Self {
        let saved_value = *val_ref;
        *val_ref = new_value;
        PushValue { saved_value, val_ref }
    }
}

impl<'a, T: Copy> Drop for PushValue<'a, T> {
    fn drop(&mut self) {
        *self.val_ref = self.saved_value
    }
}

pub struct WidgetS<'a> {
    bhv: Box<dyn WidgetBehavior>,
    layout: Option<Box<dyn WidgetLayout<'a>>>,
    children: Vec<WidgetS<'a>>
}

pub trait WidgetBehavior {
    fn draw_self(&self, offset: &Point, ctx: &mut WidgetDrawCtx);
    fn click_self(&self, offset: &Point, ctx: &mut WidgetEventCtx);
    fn hover_self(&self, offset: &Point, ctx: &mut WidgetEventCtx);
    fn measure(&self, ctx: &DrawCtx) -> Point;
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.measure(ctx)
    }
    fn selection(&self) -> Option<Box<dyn SelectionT>> {
        None
    }
}

impl<'a> WidgetS<'a> {
    fn draw(&'a self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        self.bhv.draw_self(offset, ctx);
        if let Some(ref layout) = self.layout {
            layout.draw_l(&self.children, offset, ctx);
        }
    }
    fn click(&'a mut self, offset: &Point, ctx: &mut WidgetEventCtx) {
        self.bhv.click_self(offset, ctx);
        if let Some(ref mut layout) = self.layout {
            layout.click_l(&mut self.children, offset, ctx);
        }
    }
    fn hover(&'a mut self, offset: &Point, ctx: &mut WidgetEventCtx) {
        self.bhv.hover_self(offset, ctx);
        if let Some(ref mut layout) = self.layout {
            layout.hover_l(&mut self.children, offset, ctx);
        }
    }
    fn push(&'a mut self, child: WidgetS<'a>) -> &mut Self {
        self.children.push(child);
        self
    }
}

pub trait WidgetLayout<'a> {
    fn rects(&'a self) -> Box<dyn Iterator<Item=&'a Rect> + 'a>;
    fn measure_items_l(&self, ctx: &DrawCtx) -> Point;
    fn remeasure_items_l(&mut self, widgets: &mut Vec<WidgetS>, ctx: &DrawCtx) -> Point;
    fn draw_l(&'a self, widgets: &'a Vec<WidgetS<'a>>, offset: &Point, ctx: &mut WidgetDrawCtx) {
        //self.draw_self(offset, ctx);
        for (w, r) in widgets.iter().zip(self.rects()) {
            w.draw(&(*offset + r.c1), ctx.next_widget_ctx(false));
        }
    }
    fn click_l(&'a mut self, widgets: &'a mut Vec<WidgetS<'a>>, off_pt: &Point, ctx: &mut WidgetEventCtx) {
        let w_idx = ctx.widget_idx;
        for (c_idx, (w, r)) in widgets.iter_mut().zip(self.rects()).enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                //println!("Clicked in bounds! {:?} {:?}", w_idx, c_idx);
                w.click(&(*off_pt - r.c1), ctx.child_ctx(w_idx, c_idx, false));
                break;
            }
        }
    }
    fn hover_l(&'a mut self, widgets: &'a mut Vec<WidgetS<'a>>, off_pt: &Point, ctx: &mut WidgetEventCtx) {
        let w_idx = ctx.widget_idx;
        for (c_idx, (w, r)) in widgets.iter_mut().zip(self.rects()).enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                w.hover(&(*off_pt - r.c1), ctx.child_ctx(w_idx, c_idx, false));
                break;
            }
        }
    }
}

impl<'a> WidgetLayout<'a> for WidgetList {
    fn measure_items_l(&self, _: &DrawCtx) -> Point {
        self.size
    }
    fn remeasure_items_l(&mut self, widgets: &mut Vec<WidgetS>, ctx: &DrawCtx) -> Point {
        let mut off = Point::origin();
        let mut size = Point::origin();
        for (i, w) in widgets.iter_mut().enumerate() {
            let spacing = if i == 0 { 0. } else { self.spacing as f32 };
            let m = w.bhv.remeasure(ctx);
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
            self.widget_rects[i] = Rect {
                c1: off,
                c2: off + m,
            };
        }
        *self.needs_draw.borrow_mut() = vec![true; self.widgets.len()];
        self.size = size;
        size
    }
    fn rects(&'a self) -> Box<dyn Iterator<Item=&'a Rect> + 'a> {
        Box::new(self.widget_rects.iter())
    }
}

impl<'a> WidgetLayout<'a> for WidgetGrid {
    fn measure_items_l(&self, _: &DrawCtx) -> Point {
        self.size
    }
    fn remeasure_items_l(&mut self, widgets: &mut Vec<WidgetS>, ctx: &DrawCtx) -> Point {
        if let Some(max_n_col) = self.rows.iter().max_by_key(|r| r.len()).map(|r| r.len()) {
            let mut max_col_widths: Vec<f32> = vec![0.; max_n_col];
            let mut row_heights: Vec<f32> = vec![0.; self.rows.len()];
            for (r, row) in self.rows.iter_mut().enumerate() {
                for (c, widget) in row.iter_mut().enumerate() {
                    let m = widget.remeasure(ctx);
                    self.widget_rects[r][c] = Rect {
                        c1: Point::origin(),
                        c2: m,
                    };
                    max_col_widths[c] = max_col_widths[c].max(m.x);
                    row_heights[r] = row_heights[r].max(m.y);
                }
            }
            let mut row_offset = 0.;
            for (r, rect_row) in self.widget_rects.iter_mut().enumerate() {
                let mut col_offset = 0.;
                for (c, rect) in rect_row.iter_mut().enumerate() {
                    rect.set_offset(&Point::new(col_offset, row_offset));
                    col_offset += max_col_widths[c] + self.spacing.x;
                }
                row_offset += row_heights[r] + self.spacing.y;
                self.size.x = self.size.x.max(col_offset);
            }
            self.size.y = self.size.y.max(row_offset);
            return self.size;
        }
        Point::origin()
    }
    fn rects(&'a self) -> Box<dyn Iterator<Item=&'a Rect> + 'a> {
        Box::new(self.widget_rects.iter().flatten())
    }
}
