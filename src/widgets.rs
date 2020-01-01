/*extern crate nalgebra_glm;
extern crate bitflags;*/

use crate::interface::{CallbackFn, EventCtx};
use crate::primitives::{
    rgb_to_f32, Border, BorderRect, DrawCtx, InBounds, Point, Radians, Rect, RotateRect,
};
use crate::render_text::TextParams;
use crate::textedit::TextBox;
use crate::widgets::EventResponse::*;
use bitflags::bitflags;
//use chrono::Datelike;
use nalgebra_glm as glm;
use sdl2::keyboard::Keycode;
use sdl2::mouse::SystemCursor;
use std::any::Any;
//use std::cell::RefCell;
use std::rc::Rc;

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

pub enum EventResponse {
    Handled,
    NotHandled,
}

bitflags! {
    pub struct WidgetStatus: u32 {
        const FINE = 0;
        const REDRAW = 1;
        const REMEASURE = 3;
    }
}

pub struct WidgetList {
    pub orientation: Orientation,
    pub spacing: u32,
    pub size: Point,
    widget_rects: Vec<Rect>,
}

impl WidgetList {
    pub fn new(orientation: Orientation, spacing: u32) -> Self {
        WidgetList {
            orientation,
            spacing,
            widget_rects: Vec::new(),
            size: Point::origin(),
        }
    }
}

pub struct WidgetGrid {
    widget_rects: Vec<Rect>,
    n_cols: usize,
    spacing: Point,
    size: Point,
}

impl WidgetGrid {
    pub fn new(n_cols: usize, spacing: Point) -> Self {
        WidgetGrid {
            widget_rects: Vec::new(),
            n_cols,
            spacing,
            size: Point::origin(),
        }
    }
}

pub fn new_label<T: Into<String>>(text: T) -> WidgetS {
    Label::new(text, None, None, None, TextParams::new())
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

pub fn new_textbox(default_text: &str, num_chars: usize) -> WidgetS {
    new_widget(TextBox::new(default_text, num_chars))
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
    fn on_select(&mut self, _: &mut EventCtx) -> Option<EventResponse> {
        //self.is_focus = true;
        None
    }
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<EventResponse> {
        //self.is_focus = false;
        //if !self.open {
        None
        //} else {
        //   self.open = false;
        //  Some(just_status(WidgetStatus::REMEASURE))
        //}
    }
    fn handle_key_down(&mut self, kc: &Keycode, _: &mut EventCtx) -> Option<EventResponse> {
        match *kc {
            Keycode::Down => {
                if self.selected < self.max_value {
                    self.selected += 1;
                    Some(Handled)
                } else {
                    None
                }
            }
            Keycode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Some(Handled)
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

pub type SelectMap = Vec<Option<Box<dyn SelectionT>>>;
pub type ChildrenSizes = Vec<Vec<usize>>;
//pub type WidgetSizeMap = Vec<Option<Point>>;

fn children_recurse(cur: &WidgetS, pos: &mut usize, vec: &mut ChildrenSizes) -> usize {
    let mut size = 1;
    let idx = *pos;
    *pos += 1;
    vec.push(Vec::new());
    for c in cur.children() {
        let csize = children_recurse(c, pos, vec);
        vec[idx].push(csize);
        size += csize;
    }
    size
}

fn children_sizes(root: &WidgetS) -> ChildrenSizes {
    let mut vec: ChildrenSizes = Vec::new();
    let mut pos = 0;
    children_recurse(root, &mut pos, &mut vec);
    vec
}

pub trait SelectionT {
    fn on_select(&mut self, ctx: &mut EventCtx) -> Option<EventResponse>;
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<EventResponse> {
        None
    }
    fn handle_key_down(&mut self, _: &Keycode, _: &mut EventCtx) -> Option<EventResponse> {
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
    widget_idx: Vec<Option<usize>>,
}

impl SelectionList {
    fn recurse_build(
        cur: &WidgetS,
        pos: &mut usize,
        v: &mut Vec<Box<dyn SelectionT>>,
        widget_idx: &mut Vec<Option<usize>>,
    ) {
        if let Some(select) = cur.bhv.selection() {
            widget_idx.push(Some(v.len()));
            v.push(select);
        } else {
            widget_idx.push(None);
        }
        *pos += 1;
        for w in cur.children() {
            SelectionList::recurse_build(w, pos, v, widget_idx);
        }
    }
    fn new(root: &WidgetS) -> Self {
        let mut vec: Vec<Box<dyn SelectionT>> = Vec::new();
        let mut widget_idx: Vec<Option<usize>> = Vec::new();
        let mut pos = 0;
        SelectionList::recurse_build(root, &mut pos, &mut vec, &mut widget_idx);
        SelectionList { vec, widget_idx }
    }
}

pub struct SelectionState {
    list: SelectionList,
    cur_select: Option<usize>,
    child_sizes: ChildrenSizes,
}

#[derive(Clone, Copy, Debug)]
pub struct WidgetIdx(pub usize);

impl SelectionState {
    pub fn new(root: &WidgetS) -> Self {
        SelectionState {
            list: SelectionList::new(root),
            cur_select: None,
            child_sizes: children_sizes(root),
        }
    }
    pub fn is_select(&self) -> bool {
        self.cur_select.is_some()
    }
    pub fn set_select(&mut self, idx: Option<usize>, ctx: &mut EventCtx) {
        self.cur_select
            .and_then(|idx| self.list.vec[idx].on_deselect(ctx));
        self.cur_select = idx;
        self.cur_select
            .and_then(|idx| self.list.vec[idx].on_select(ctx));
    }
    pub fn get(&self, idx: usize) -> &Box<dyn SelectionT> {
        &self.list.vec[idx]
    }
    pub fn get_mut(&mut self, idx: usize) -> &mut Box<dyn SelectionT> {
        &mut self.list.vec[idx]
    }
    pub fn select_next(&mut self, ctx: &mut EventCtx) -> Option<EventResponse> {
        if let Some(ref mut idx) = self.cur_select {
            if *idx < self.list.vec.len() - 1 {
                *idx += 1;
                return self.on_select(ctx);
            }
        }
        None
    }
    pub fn get_select<'a, T: SelectionT + 'static>(&'a self, idx: usize) -> Option<&'a T> {
        self.list.vec[idx]
            .as_any()
            .and_then(|s| s.downcast_ref::<T>())
    }
    pub fn get_select_mut<'a, T: SelectionT + 'static>(
        &'a mut self,
        idx: usize,
    ) -> Option<&'a mut T> {
        self.list.vec[idx]
            .as_any_mut()
            .and_then(|s| s.downcast_mut::<T>())
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
            self.list.vec[idx]
                .as_any()
                .and_then(|s| s.downcast_ref::<T>())
        } else {
            None
        }
    }
    pub fn get_select_w_mut<'a, T: SelectionT + 'static>(
        &'a mut self,
        widx: WidgetIdx,
    ) -> Option<&'a mut T> {
        if let Some(idx) = self.list.widget_idx[widx.0] {
            self.list.vec[idx]
                .as_any_mut()
                .and_then(|s| s.downcast_mut::<T>())
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
    fn on_select(&mut self, ctx: &mut EventCtx) -> Option<EventResponse> {
        self.cur_select
            .and_then(|idx| self.list.vec[idx].on_select(ctx))
    }
    fn on_deselect(&mut self, ctx: &mut EventCtx) -> Option<EventResponse> {
        self.cur_select
            .and_then(|idx| self.list.vec[idx].on_deselect(ctx))
    }
    fn handle_key_down(&mut self, kc: &Keycode, ctx: &mut EventCtx) -> Option<EventResponse> {
        self.cur_select
            .and_then(|idx| self.list.vec[idx].handle_key_down(kc, ctx))
    }
    fn log(&self) {}
}

pub struct WidgetDrawCtx<'a> {
    pub draw_ctx: &'a DrawCtx,
    pub select_state: &'a SelectionState,
    pub widget_idx: WidgetIdx,
}

impl<'a> WidgetDrawCtx<'a> {
    pub fn new(draw_ctx: &'a DrawCtx, select_state: &'a SelectionState) -> Self {
        WidgetDrawCtx {
            draw_ctx,
            select_state,
            widget_idx: WidgetIdx(0),
        }
    }
    fn next_widget_ctx(&mut self) -> &mut Self {
        self.widget_idx.0 += 1;
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

pub struct EventResult {
    pub cursor: SystemCursor,
    pub callbacks: Vec<CallbackFn>,
    pub status: WidgetStatus,
}

impl EventResult {
    pub fn new() -> Self {
        EventResult {
            cursor: SystemCursor::Arrow,
            callbacks: Vec::new(),
            status: WidgetStatus::FINE
        }
    }
}

pub struct WidgetEventCtx<'a> {
    pub draw_ctx: &'a DrawCtx,
    pub select_state: &'a SelectionState,
    pub widget_idx: WidgetIdx,
    pub res: EventResult
}

impl<'a> WidgetEventCtx<'a> {
    pub fn new(
        draw_ctx: &'a DrawCtx,
        select_state: &'a SelectionState,
    ) -> Self {
        WidgetEventCtx {
            draw_ctx,
            select_state,
            widget_idx: WidgetIdx(0),
            res: EventResult::new()
        }
    }
    fn child_ctx(&mut self, parent_idx: WidgetIdx, child_pos: usize) -> &mut Self {
        self.widget_idx = self.select_state.child_widget_idx(parent_idx, child_pos);
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
    pub fn push_cb(&mut self, cb: CallbackFn) {
        self.res.callbacks.push(cb);
    }
    pub fn set_cursor(&mut self, cursor: SystemCursor) {
        self.res.cursor = cursor;
    }
    /*fn next_widget_ctx(&mut self, is_leaf: bool) -> &mut Self {
        if !is_leaf {
            self.widget_idx.0 += 1;
        }
        self
    }*/
}

pub type WidgetSizes = Vec<Option<Point>>;

#[allow(dead_code)]
pub struct MeasureContext<'a> {
    draw_ctx: &'a DrawCtx,
    sizes: &'a mut WidgetSizes,
    widget_idx: WidgetIdx
}

#[allow(dead_code)]
impl<'a> MeasureContext<'a> {
    pub fn new(draw_ctx: &'a DrawCtx, sizes: &'a mut WidgetSizes) -> Self {
        MeasureContext {
            draw_ctx,
            sizes,
            widget_idx: WidgetIdx(0)
        }
    }
    fn next_widget_ctx(&mut self) -> &mut Self {
        self.widget_idx.0 += 1;
        self
    }
    fn cur_size(&self) -> Option<Point> {
        self.sizes[self.widget_idx.0]
    }
    fn cur_size_mut(&mut self) -> &mut Option<Point> {
        &mut self.sizes[self.widget_idx.0]
    }
}

struct PushValue<'a, T: Copy> {
    val_ref: &'a mut T,
    saved_value: T,
}


#[allow(dead_code)]
impl<'a, T: Copy> PushValue<'a, T> {
    pub fn new(val_ref: &'a mut T, new_value: T) -> Self {
        let saved_value = *val_ref;
        *val_ref = new_value;
        PushValue {
            saved_value,
            val_ref,
        }
    }
}

impl<'a, T: Copy> Drop for PushValue<'a, T> {
    fn drop(&mut self) {
        *self.val_ref = self.saved_value
    }
}

#[derive(Debug)]
pub enum Visibility {
    Visible,
    Invisible,
    Collapsed,
}

pub struct WidgetProps {
    layout: Box<dyn WidgetLayout>,
    size_cache: Option<Point>,
    children_size: Point,
    visible: Visibility,
    children: Vec<WidgetS>,
    status: WidgetStatus,
}

impl WidgetProps {
    pub fn set_status(&mut self, status: WidgetStatus) {
        self.status = status;
    }
    pub fn set_remeasure(&mut self) {
        self.status |= WidgetStatus::REMEASURE
    }
    pub fn set_redraw(&mut self) {
        self.status |= WidgetStatus::REDRAW
    }
}
impl Default for WidgetProps {
    fn default() -> Self {
        WidgetProps {
            layout: Box::new(WidgetList::new(Orientation::Vertical, 10)),
            size_cache: None,
            children_size: Point::origin(),
            visible: Visibility::Visible,
            children: Vec::new(),
            status: WidgetStatus::FINE,
        }
    }
}

pub struct WidgetS {
    bhv: Box<dyn WidgetBehavior>,
    props: WidgetProps,
}

pub trait WidgetBehavior {
    fn draw_self(&self, _: &Point, _: &WidgetProps, _: &mut WidgetDrawCtx) {}
    fn click_self(
        &mut self,
        _: &Point,
        _: &mut WidgetProps,
        _: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        Some(NotHandled)
    }
    fn hover_self(
        &mut self,
        _: &Point,
        _: &mut WidgetProps,
        _: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        Some(NotHandled)
    }
    fn remeasure_self_before(&mut self, _: &mut WidgetProps, _: &DrawCtx) {}
    fn remeasure_self_after(&mut self, props: &WidgetProps, _: &DrawCtx) -> Point {
        props.children_size
    }
    fn draw(&self, off: &Point, props: &WidgetProps, ctx: &mut WidgetDrawCtx) {
        //println!("Drawing widget: {:?}", ctx.widget_idx);
        self.draw_self(off, props, ctx);
        props.layout.draw_l(&props.children, off, ctx);
    }
    fn click(
        &mut self,
        off: &Point,
        props: &mut WidgetProps,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        if let Some(NotHandled) = self.click_self(off, props, ctx) {
            props
                .layout
                .click_l(off, &mut props.children, &mut props.status, ctx)
        } else {
            None
        }
    }
    fn hover(
        &mut self,
        off: &Point,
        props: &mut WidgetProps,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        if let Some(NotHandled) = self.hover_self(off, props, ctx) {
            props
                .layout
                .hover_l(off, &mut props.children, &mut props.status, ctx)
        } else {
            None
        }
    }
    fn selection(&self) -> Option<Box<dyn SelectionT>> {
        None
    }
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
    fn deselect(&mut self, _: &mut WidgetEventCtx) {}
}

pub struct Container {}

impl WidgetBehavior for Container {}

pub fn new_widget<T: WidgetBehavior + Sized + 'static>(w: T) -> WidgetS {
    WidgetS {
        bhv: Box::new(w),
        props: WidgetProps::default(),
    }
}

pub fn new_container<T: WidgetLayout + Sized + 'static>(w: T) -> WidgetS {
    WidgetS {
        bhv: Box::new(Container {}),
        props: WidgetProps {
            layout: Box::new(w),
            children: Vec::new(),
            visible: Visibility::Visible,
            size_cache: None,
            children_size: Point::origin(),
            status: WidgetStatus::FINE,
        },
    }
}

impl WidgetS {
    pub fn draw(&self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        self.bhv.draw(offset, &self.props, ctx);
    }
    pub fn handle_response(&mut self, resp: Option<EventResponse>) -> Option<EventResponse> {
        if self.props.status & WidgetStatus::REMEASURE != WidgetStatus::FINE {
            self.props.size_cache = None;
        }
        resp
    }
    pub fn click(&mut self, offset: &Point, ctx: &mut WidgetEventCtx) -> Option<EventResponse> {
        let resp = self.bhv.click(offset, &mut self.props, ctx);
        self.handle_response(resp)
    }
    pub fn hover(&mut self, offset: &Point, ctx: &mut WidgetEventCtx) -> Option<EventResponse> {
        let resp = self.bhv.hover(offset, &mut self.props, ctx);
        self.handle_response(resp)
    }
    pub fn size_cache(&self) -> Option<Point> {
        //println!("Size cache value: {:?}", self.props.size_cache);
        self.props.size_cache
    }
    pub fn needs_measure(&self) -> bool {
        self.props.size_cache.is_none()
    }
    pub fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        //println!("Remeasuring!");
        self.bhv.remeasure_self_before(&mut self.props, ctx);
        self.props.children_size = self
            .props
            .layout
            .remeasure_items_l(&mut self.props.children, ctx);
        let size = self.bhv.remeasure_self_after(&self.props, ctx);
        self.props.size_cache = Some(size);
        //println!("Size: {:?}", size);
        size
    }
    pub fn children(&self) -> &Vec<WidgetS> {
        &self.props.children
    }
    pub fn children_mut(&mut self) -> &mut Vec<WidgetS> {
        &mut self.props.children
    }
    pub fn deselect(&mut self, ctx: &mut WidgetEventCtx) {
        self.bhv.deselect(ctx);
        for c in &mut self.props.children {
            c.deselect(ctx);
        }
    }
    pub fn push(&mut self, child: WidgetS) -> &mut Self {
        self.props.children.push(child);
        self
    }
    pub fn status(&self) -> WidgetStatus {
        self.props.status
    }
    pub fn set_layout(&mut self, layout: Box<dyn WidgetLayout>) -> &mut Self {
        self.props.layout = layout;
        self
    }
    pub fn set_visible(&mut self, visible: Visibility) -> &mut Self {
        self.props.visible = visible;
        self
    }
    pub fn downcast<'a, W: WidgetBehavior + 'static>(&'a self) -> &'a W {
        self.bhv.as_any().unwrap().downcast_ref::<W>().unwrap()
    }
    pub fn downcast_mut<'a, W: WidgetBehavior + 'static>(&'a mut self) -> &'a mut W {
        self.bhv.as_any_mut().unwrap().downcast_mut::<W>().unwrap()
    }
}

impl std::ops::Add<WidgetS> for WidgetS {
    type Output = WidgetS;
    fn add(mut self, rhs: WidgetS) -> Self::Output {
        self.props.children.push(rhs);
        self
    }
}

impl std::ops::AddAssign<WidgetS> for WidgetS {
    fn add_assign(&mut self, rhs: WidgetS) {
        self.props.children.push(rhs);
    }
}

impl std::ops::Add<Vec<WidgetS>> for WidgetS {
    type Output = WidgetS;
    fn add(mut self, rhs: Vec<WidgetS>) -> Self::Output {
        for c in rhs {
            self.props.children.push(c);
        }
        self
    }
}

impl std::ops::AddAssign<Vec<WidgetS>> for WidgetS {
    fn add_assign(&mut self, rhs: Vec<WidgetS>) {
        for c in rhs {
            self.props.children.push(c);
        }
    }
}

pub trait WidgetLayout {
    fn rects<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Rect> + 'a>;
    fn remeasure_items_l(&mut self, widgets: &mut Vec<WidgetS>, ctx: &DrawCtx) -> Point;
    fn get_idx(&self, offset: &Point, ctx: &DrawCtx) -> Option<usize> {
        self.rects()
            .position(|r| r.in_bounds(offset, &ctx.viewport))
    }
    fn draw_l(&self, widgets: &Vec<WidgetS>, offset: &Point, ctx: &mut WidgetDrawCtx) {
        //println!("Drawing Layout {:?}", widgets.len());
        for (w, r) in widgets.iter().zip(self.rects()) {
            let c_ctx = ctx.next_widget_ctx();
            if let Visibility::Visible = w.props.visible {
                w.draw(&(*offset + r.c1), c_ctx);
            }
        }
    }
    fn click_l<'a>(
        &'a self,
        off_pt: &Point,
        widgets: &'a mut Vec<WidgetS>,
        status: &mut WidgetStatus,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        let w_idx = ctx.widget_idx;
        let mut resp: Option<EventResponse> = None;
        for (c_idx, (w, r)) in widgets.iter_mut().zip(self.rects()).enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                //println!("Clicked in bounds! {:?} {:?}", w_idx, c_idx);
                resp = w.click(&(*off_pt - r.c1), ctx.child_ctx(w_idx, c_idx));
                *status |= w.status();
                break;
            }
        }
        resp
    }
    fn hover_l<'a>(
        &'a self,
        off_pt: &Point,
        widgets: &'a mut Vec<WidgetS>,
        status: &mut WidgetStatus,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        let w_idx = ctx.widget_idx;
        let mut resp: Option<EventResponse> = None;
        for (c_idx, (w, r)) in widgets.iter_mut().zip(self.rects()).enumerate() {
            if r.in_bounds(off_pt, &ctx.draw_ctx.viewport) {
                resp = w.hover(&(*off_pt - r.c1), ctx.child_ctx(w_idx, c_idx));
                *status |= w.status();
                break;
            }
        }
        resp
    }
}

impl WidgetLayout for WidgetList {
    fn remeasure_items_l(&mut self, widgets: &mut Vec<WidgetS>, ctx: &DrawCtx) -> Point {
        let mut off = Point::origin();
        let mut size = Point::origin();
        let mut first = true;
        self.widget_rects.resize(widgets.len(), Rect::empty());
        for (i, w) in widgets.iter_mut().enumerate() {
            if let Visibility::Collapsed = w.props.visible {
                continue;
            }
            let spacing = if first {
                0.
            } else {
                first = false;
                self.spacing as f32
            };
            let m = w.size_cache().unwrap_or(w.remeasure(ctx));
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
        self.size = size;
        size
    }
    fn rects<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Rect> + 'a> {
        Box::new(self.widget_rects.iter())
    }
}

impl WidgetLayout for WidgetGrid {
    fn remeasure_items_l(&mut self, widgets: &mut Vec<WidgetS>, ctx: &DrawCtx) -> Point {
        self.widget_rects.resize(widgets.len(), Rect::empty());
        let mut max_col_widths: Vec<f32> = vec![0.; self.n_cols];
        let mut row_heights: Vec<f32> = vec![0.; self.widget_rects.len() / self.n_cols + 1];
        let n_cols = self.n_cols;
        for (i, (rect, widget)) in self.widget_rects.iter_mut().zip(widgets.iter_mut()).enumerate() {
            let (r, c) = (i / n_cols, i % n_cols);
            let m = widget.size_cache().unwrap_or(widget.remeasure(ctx));
            *rect = Rect {
                c1: Point::origin(),
                c2: m,
            };
            max_col_widths[c] = max_col_widths[c].max(m.x);
            row_heights[r] = row_heights[r].max(m.y);
        }
        let mut row_offset = 0.;
        let mut col_offset = 0.;
        for (i, rect) in self.widget_rects.iter_mut().enumerate() {
            let (r, c) = (i / n_cols, i % n_cols);
            rect.set_offset(&Point::new(col_offset, row_offset));
            col_offset += max_col_widths[c] + self.spacing.x;
            if i == n_cols - 1 {
                self.size.x = self.size.x.max(col_offset);
                row_offset += row_heights[r] + self.spacing.y;
                col_offset = 0.;
            }
        }
        //println!("Widget rects: {:?}", self.widget_rects);
        self.size.y = self.size.y.max(row_offset);
        self.size
    }
    fn rects<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Rect> + 'a> {
        Box::new(self.widget_rects.iter())
    }
}

pub fn new_button(border: Border, fill_color: glm::Vec4, onclick: CallbackFn) -> WidgetS {
    new_widget(Button::new(border, fill_color, onclick))
}

pub struct Button {
    pub onclick: CallbackFn,
    border_rect: BorderRect,
}

impl Button {
    pub fn new(border: Border, fill_color: glm::Vec4, onclick: CallbackFn) -> Self {
        let border_rect = BorderRect::new(Point::origin(), fill_color, border);
        Button {
            onclick,
            border_rect,
        }
    }
}

impl WidgetBehavior for Button {
    fn hover_self(
        &mut self,
        _: &Point,
        _: &mut WidgetProps,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        ctx.set_cursor(SystemCursor::Hand);
        Some(Handled)
    }
    fn draw_self(&self, offset: &Point, _: &WidgetProps, ctx: &mut WidgetDrawCtx) {
        self.border_rect.draw(*offset, ctx.draw_ctx);
    }
    fn click_self(
        &mut self,
        _: &Point,
        _: &mut WidgetProps,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        ctx.push_cb(Rc::clone(&self.onclick));
        Some(Handled)
    }
    fn remeasure_self_after(&mut self, props: &WidgetProps, _: &DrawCtx) -> Point {
        self.border_rect.size = props.children_size;
        props.children_size + self.border_rect.border.width * Point::new(2., 2.)
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
    pub fn new<T: Into<String>>(
        text: T,
        bg_color: Option<glm::Vec4>,
        hover_color: Option<glm::Vec4>,
        min_width: Option<f32>,
        text_params: TextParams,
    ) -> WidgetS {
        new_widget(Label {
            text: text.into(),
            bg_color,
            hover_color,
            min_width,
            is_hover: false,
            text_params,
        })
    }
}

impl Label {
    fn set_hover(&mut self, h: bool) {
        self.is_hover = h;
    }
}

impl WidgetBehavior for Label {
    fn remeasure_self_after(&mut self, _: &WidgetProps, ctx: &DrawCtx) -> Point {
        let mut m = ctx.render_text.measure(&self.text, self.text_params.scale);
        if let Some(min_width) = self.min_width {
            m.x = m.x.max(min_width);
        }
        m
    }
    fn draw_self(&self, offset: &Point, _: &WidgetProps, ctx: &mut WidgetDrawCtx) {
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
    fn hover_self(
        &mut self,
        _: &Point,
        props: &mut WidgetProps,
        _: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        if self.hover_color.is_some() && !self.is_hover {
            self.is_hover = true;
            props.set_redraw();
        }
        Some(Handled)
    }
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

pub struct DropDown {
    selected: usize,
    hover_idx: Option<usize>,
    open: bool,
    n_items: usize,
}

pub fn new_dropdown<T: Into<String> + AsRef<str>>(values: Vec<T>, selected: usize) -> WidgetS {
    DropDown::new(values, selected)
}

impl DropDown {
    pub fn new<T: Into<String> + AsRef<str>>(values: Vec<T>, selected: usize) -> WidgetS {
        let white = rgb_to_f32(255, 255, 255);
        let lb = rgb_to_f32(168, 238, 240);
        let mut children: Vec<WidgetS> = Vec::new();
        for v in values.into_iter() {
            children.push(Label::new(
                v,
                Some(white),
                Some(lb),
                None,
                TextParams::new(),
            ));
        }
        let layout = Box::new(WidgetList::new(Orientation::Vertical, 0));
        WidgetS {
            bhv: Box::new(DropDown {
                selected,
                hover_idx: None,
                n_items: children.len(),
                open: false,
            }),
            props: WidgetProps { children, layout, ..WidgetProps::default() }
        }
    }
}

impl WidgetBehavior for DropDown {
    fn remeasure_self_before(&mut self, props: &mut WidgetProps, ctx: &DrawCtx) {
        let char_size = ctx.render_text.char_size('a', 1.0);
        let mut max_width: f32 = 0.;
        for c in props.children.iter() {
            let text = &c.downcast::<Label>().text;
            max_width = max_width.max(ctx.render_text.measure(text, 1.0).x);
        }
        max_width += char_size.x;
        for c in props.children.iter_mut() {
            c.downcast_mut::<Label>().min_width = Some(max_width);
        }
        for (i, c) in props.children.iter_mut().enumerate() {
            let vis = if self.open || i == self.selected { Visibility::Visible } 
                else { Visibility::Collapsed };
            c.set_visible(vis);
        }
    }
    fn draw(&self, off: &Point, props: &WidgetProps, ctx: &mut WidgetDrawCtx) {
        props.layout.draw_l(&props.children, off, ctx);
        let char_size = ctx.draw_ctx.render_text.char_size('a', 1.0);
        let blue = glm::vec4(0., 0., 1., 1.);
        let tri_center = Point::new(off.x + props.children_size.x
            - char_size.x / 2., off.y + char_size.y / 2.);
        ctx.draw_ctx.draw_iso_tri(
            tri_center,
            char_size.x,
            char_size.y,
            blue,
            true,
            Radians(std::f32::consts::PI),
        );
    }
    fn click(
        &mut self,
        off: &Point,
        props: &mut WidgetProps,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        if self.open {
            self.selected = props.layout.get_idx(off, ctx.draw_ctx).unwrap_or(0);
            //println!("Selected! {:?}", self.selected);
        }
        self.open = !self.open;
        ctx.res.status |= WidgetStatus::REMEASURE;
        //props.set_remeasure();
        Some(Handled)
    }
    fn hover_self(
        &mut self,
        off: &Point,
        props: &mut WidgetProps,
        ctx: &mut WidgetEventCtx,
    ) -> Option<EventResponse> {
        if self.open {
            if let Some(hover_idx) = props.layout.get_idx(off, ctx.draw_ctx) {
                let mut change = true;
                if let Some(prev_hover) = self.hover_idx {
                    change = prev_hover != hover_idx;
                    props.children[prev_hover]
                        .downcast_mut::<Label>()
                        .set_hover(false);
                }
                if change {
                    println!("Hover idx: {:?}", hover_idx);
                    props.children[hover_idx]
                        .downcast_mut::<Label>()
                        .set_hover(true);
                    self.hover_idx = Some(hover_idx);
                    ctx.res.status |= WidgetStatus::REDRAW;
                }
            }
        }
        Some(Handled)
    }
    fn deselect(&mut self, _: &mut WidgetEventCtx) {
        if self.open {
            self.open = false;
        }
    }
    fn selection(&self) -> Option<Box<dyn SelectionT>> {
        Some(Box::new(DropDownSelect::new(self.selected, self.n_items)))
    }
}
