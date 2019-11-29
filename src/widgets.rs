extern crate nalgebra_glm;
extern crate bitflags;

use crate::interface::{CallbackFn, EventCtx, AppState};
use crate::render_text::{TextParams};
use crate::primitives::{DrawCtx, Point, Rect, RotateRect, Radians, Border, BorderRect, InBounds, rgb_to_f32};
use nalgebra_glm as glm;
use bitflags::bitflags;
use std::rc::Rc;
use std::cell::RefCell;

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
    pub fn add(&mut self, w: Box<dyn Widget>, ctx: &DrawCtx) -> usize {
        let m = w.measure(ctx);
        let spacing = if self.widgets.is_empty() { 0. } else { self.spacing as f32 };
        self.widgets.push(w);
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
        self.widgets.len() - 1
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
    fn handle_response(&mut self, resp: Option<(usize, WidgetResponse)>) -> Option<WidgetResponse> {
        if let Some((/*i*/_, (status, _))) = resp {
            /*match status {
                REDRAW | REMEASURE => {
                    self.needs_draw.borrow_mut()[i] = true;
                    self.needs_draw.borrow_mut() = vec![true; self.widgets.len()];
                }
                _ => {}
            };*/
            return Some(resp.unwrap().1)
        }
        None
    }
}

impl Widget for WidgetList {
    fn measure(&self, _: &DrawCtx) -> Point {
        self.size
    }
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        //let mut needs_draw = self.needs_draw.borrow_mut();
        for (i, w) in self.widgets.iter().enumerate() {
            //if needs_draw[i] {
                w.draw(&(*offset + self.widget_rects[i].c1), ctx);
             //   needs_draw[i] = false;
           //}
        }
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        self.widgets.iter_mut().fold(None, |resp, w| { 
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
        let mut resp: Option<(usize, WidgetResponse)> = None;
        for (i, w) in self.widgets.iter_mut().enumerate() {
            let rect = &self.widget_rects[i];
            if rect.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = w.click(&(*off_pt - rect.c1), ctx).map(|r| (i, r));
                break;
            }
        }
        if let Some((i, _)) = resp {
            for (j, w) in self.widgets.iter_mut().enumerate() {
                if i != j {
                    if let Some(wr2) = w.deselect() {
                        println!("Deselecting and combining, status: {:?}", wr2.0);
                        resp = Some((i, combine_response(&resp.unwrap().1, &wr2)));
                    }
                }
            }
        }
        self.handle_response(resp)
    }
    fn hover(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        let mut resp: Option<(usize, WidgetResponse)> = None;
        for (i, w) in self.widgets.iter_mut().enumerate() {
            let rect = &self.widget_rects[i];
            if rect.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = w.hover(&(*off_pt - rect.c1), ctx).map(|r| (i, r));
                break;
            }
        }
        self.handle_response(resp)
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
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
}

pub struct Label {
    text: &'static str,
    bg_color: Option<glm::Vec4>,
    hover_color: Option<glm::Vec4>,
    is_hover: bool,
    min_width: Option<f32>,
    text_params: TextParams,
}

impl Label {
    pub fn new(text: &'static str, bg_color: Option<glm::Vec4>, hover_color: Option<glm::Vec4>,
               min_width: Option<f32>, text_params: TextParams) -> Self {
        Label { text, bg_color, hover_color, min_width, is_hover: false, text_params }
    }
}

impl Widget for Label {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        let mut m = ctx.render_text.measure(self.text, self.text_params.scale);
        if let Some(min_width) = self.min_width {
            m.x = m.x.max(min_width);
        }
        m
    }
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        let mut m = ctx.render_text.measure(self.text, self.text_params.scale);
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
        if !self.is_hover {
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
}

pub struct DropDown {
    selected: usize,
    hover_idx: usize,
    values_list: WidgetList,
    open: bool,
}

impl DropDown {
    pub fn new(values: Vec<&'static str>, ctx: &DrawCtx) -> Self {
        let mut values_list = WidgetList::new(Orientation::Vertical, 0);
        let white = rgb_to_f32(255, 255, 255);
        let lb = rgb_to_f32(168, 238, 240);
        let mut max_width: f32 = 0.;
        for v in values.iter() {
            max_width = max_width.max(ctx.render_text.measure(v, 1.0).x);
        }
        for v in values.iter() {
            values_list.add(Box::new(Label::new(v, Some(white), Some(lb), Some(max_width), TextParams::new())), ctx);
        }
        DropDown { values_list, selected: 0, hover_idx: 0, open: false }
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
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        if self.open {
            self.selected = self.values_list.get_idx(off_pt, ctx.draw_ctx).unwrap_or(0);
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
    border_rect: BorderRect 
}

impl Button {
    pub fn new(text: &'static str, text_params: TextParams, 
        border: Border, fill_color: glm::Vec4, onclick: WidgetResponse, ctx: &DrawCtx) -> Self 
    {
        let size = ctx.render_text.measure(text, text_params.scale);
        let border_rect = BorderRect::new(size, fill_color, border);
        Button { text, text_params, onclick, border_rect }
    }
}
impl Widget for Button {
    fn measure(&self, _: &DrawCtx) -> Point {
       self.border_rect.size + self.border_rect.border.width * Point::new(2., 2.)
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