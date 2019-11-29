extern crate sdl2;

use std::collections::{HashMap};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::{Cursor, SystemCursor};
use sdl2::video::Window;
use std::time::SystemTime;
use crate::primitives::*;
//use crate::primitives::ShapeProps as Shape;
use crate::textedit::{TextBox, get_char_from_keycode, get_dir_from_keycode};
use crate::render_text::{TextParams};
use std::rc::Rc;
use std::cell::RefCell;
//use crate::hexcolor::HexColor;
use crate::widgets::*;

pub struct CursorMap(HashMap<SystemCursor, Cursor>);
impl CursorMap {
    fn new() -> Self {
        let mut m = HashMap::new();
        m.insert(SystemCursor::Arrow, Cursor::from_system(SystemCursor::Arrow).unwrap());
        m.insert(SystemCursor::Hand, Cursor::from_system(SystemCursor::Hand).unwrap());
        m.insert(SystemCursor::Crosshair, Cursor::from_system(SystemCursor::Crosshair).unwrap());
        m.insert(SystemCursor::SizeNESW, Cursor::from_system(SystemCursor::SizeNESW).unwrap());
        m.insert(SystemCursor::SizeNS, Cursor::from_system(SystemCursor::SizeNS).unwrap());
        m.insert(SystemCursor::SizeNWSE, Cursor::from_system(SystemCursor::SizeNWSE).unwrap());
        m.insert(SystemCursor::SizeWE, Cursor::from_system(SystemCursor::SizeWE).unwrap());
        m.insert(SystemCursor::IBeam, Cursor::from_system(SystemCursor::IBeam).unwrap());
        CursorMap(m)
    }
    fn get(&self, cursor: &SystemCursor) -> &Cursor {
        &self.0[cursor]
    }
}

#[allow(dead_code)]
impl Shape {
    pub fn drag(&mut self, off: &Point) {
        match self {
            Shape::Line(ref mut draw_line) => {
                draw_line.p1 += *off;
                draw_line.p2 += *off;
            }
            Shape::Polygon(ref mut draw_poly) => {
                draw_poly.rect.drag(off);
            }
        }
    }
    fn click(&self, p: &Point, vp: &Point) -> ClickResponse {
        match self.in_bounds(p, vp) {
            true => {
                ClickResponse::Clicked
            }
            false => {
                ClickResponse::NotClicked
            }
        }
    }
    fn in_select_box(&self, r: &Rect, vp: &Point) -> bool {
        self.verts(&vp).iter().any(|v| r.in_bounds(v, vp))
    }
    fn set_rect(&mut self, r: &RotateRect) {
        match self {
            Shape::Polygon(ref mut draw_poly) => {
                draw_poly.rect = r.clone();
            }
            Shape::Line(_) => { }
        }
    }
    /*fn drag_vertex(&mut self, v: &LineVertex, pt: &Point) {
        match self {
            Shape::Line(ref mut draw_line) => {
                draw_line.drag_vertex(v, pt);
            }
            Shape::Polygon(_) => { }
        }
    }*/
}

/*impl DrawLine {
    fn drag_vertex(&mut self, v: &LineVertex, pt: &Point) {
        match v {
            LineVertex::P1 => self.p1 = *pt,
            LineVertex::P2 => self.p2 = *pt,
        };
    }
}*/

type ShapeID = u32;

enum AppMode {
    ModeDefault
}

pub struct AppState {
    pub interface: WidgetList,
    pub key_item: Option<HandleKeyItem>,
    pub draw_ctx: DrawCtx,
    app_mode: AppMode,
    drag_mode: DragMode,
    key_mode: KeyboardMode,
    hover_item: HoverItem,
    window: Window,
    cursors: CursorMap,
    needs_draw: bool,
}

#[derive(Clone, Copy)]
pub enum DragMode {
    DragNone,
    /*SelectBox {start_pt: Point, last_pt: Point},
    CreateShape { shape_id: ShapeBarShape, start_pt: Point, last_pt: Point },
    DragShapes { last_pt: Point, click_shape: ShapeID, clear_select: bool },
    DragResize { click_box: ShapeID, drag_vertex: DragVertex },
    DragRotate { click_box: ShapeID, last_angle: Radians },
    DragLineVertex { shape_id: ShapeID, line_vertex: LineVertex }*/
}

#[derive(Clone, Copy)]
pub enum KeyboardMode {
    KeyboardNone,
    TextEdit(ShapeID, SystemTime),
}

#[derive(PartialEq, Clone)]
pub enum HoverItem {
   HoverNone,
   HoverText(ShapeID, usize),
}

pub type CallbackFn = Rc<dyn Fn(&mut AppState)>;

/*pub struct Interface {
    wl: WidgetList,
    needs_draw: bool
}

impl Interface {
    fn new(ctx: &DrawCtx) -> Self {
        let mut wl = WidgetList::new(Orientation::Vertical, 5);
        wl.add(Box::new(new_form(ctx)), ctx);
        Interface { wl, needs_draw: true }
    }
}

impl Widget for Interface {
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
    fn deselect(&mut self) { 
        self.wl.deselect()
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
        self.wl.remeasure(ctx)
    }
}*/

fn new_interface(ctx: &DrawCtx) -> WidgetList {
    let mut wl = WidgetList::new(Orientation::Vertical, 5);
    wl.add(Box::new(new_form(ctx)), ctx);
    wl
}

pub trait HandleKey {
    fn handle_key_down(&mut self, kc: &Keycode, rt: &EventCtx) -> Option<WidgetResponse>;
}

pub type HandleKeyItem = Rc<RefCell<dyn HandleKey>>;

impl AppState {
    pub fn new(viewport: &Point, window: Window) -> AppState {
        let draw_ctx = DrawCtx::new(viewport);
        let interface = new_interface(&draw_ctx);
        AppState {
            app_mode: AppMode::ModeDefault,
            draw_ctx,
            interface,
            window,
            drag_mode: DragMode::DragNone,
            hover_item: HoverItem::HoverNone,
            key_item: None, 
            key_mode: KeyboardMode::KeyboardNone,
            cursors: CursorMap::new(),
            needs_draw: true
        }
    }
    /*fn get_shape_select_box(&self, s: &DrawPolygon) -> ShapeSelectBox {
        ShapeSelectBox(s.rect.clone())
    }
    fn is_hover_text(&self, p: &Point, vp: &Point) -> Option<(ShapeID, usize)> {
        self.text_boxes.iter().find(|(id, _)| self.draw_list.get(id).unwrap().in_bounds(p, vp))
            .map(|(id, tb)| (id, tb, self.draw_list.get(id).unwrap().rect()))
            .and_then(|(id, tb, rect)| tb.hover_text(p, &rect, &self.draw_ctx.render_text, vp).map(|pos| (*id, pos)))
    }
    fn is_hover_select_box(&self, p: &Point, vp: &Point) -> Option<(ShapeID, BoxHover)> {
        self.selection.iter().filter_map(|(id, sb)| sb.get_hover(p, vp).map(|lh| (*id, lh))).nth(0)
    }
    fn is_hover_line(&self, p: &Point, vp: &Point) -> Option<(ShapeID, LineHover)> {
        self.line_select.iter().filter_map(|(id, l)| l.get_hover(p, vp).map(|lh| (*id, lh))).nth(0)
    }
    pub fn handle_hover_click(&mut self, pt: &Point, clear_select: bool, cursor: &mut SystemCursor) {
        match self.hover_item {
            HoverItem::HoverRect(select_id) => {
                if self.selection[&select_id].in_bounds(pt, &self.draw_ctx.viewport) {
                    self.drag_mode = DragMode::DragShapes { last_pt: *pt, click_shape: select_id, clear_select };
                    *cursor = SystemCursor::Hand;
                }
            }
            HoverItem::HoverLine(select_id) => {
                if self.line_select[&select_id].in_bounds(pt, &self.draw_ctx.viewport) {
                    self.drag_mode = DragMode::DragShapes { last_pt: *pt, click_shape: select_id, clear_select };
                    *cursor = SystemCursor::Hand;
                }
            }
            HoverItem::HoverRotate(select_id) => {
                let last_angle = self.selection[&select_id].get_rotate_angle(pt, &self.draw_ctx.viewport);
                self.drag_mode = DragMode::DragRotate { click_box: select_id, last_angle };
                    *cursor = SystemCursor::Hand;
            }
            HoverItem::HoverVertex(select_id, drag_vertex) => {
                self.drag_mode = DragMode::DragResize { click_box: select_id, drag_vertex };
                *cursor = get_drag_hover_cursor(&drag_vertex);
            }
            HoverItem::HoverShape(shape_id, ref shape) => {
                match shape_id {
                    ShapeBarShape::Line => {
                        self.hover_item = HoverItem::HoverCreateLine { start_pt: *pt, last_pt: *pt, color: shape.rgb() }
                    }
                    _ => {
                        self.drag_mode = DragMode::CreateShape {shape_id, start_pt: *pt, last_pt: *pt};
                        self.hover_item = HoverItem::HoverNone;
                    }
                }
                *cursor = SystemCursor::Crosshair;
            }
            HoverItem::HoverText(tb_id, cursor_pos) => {
                self.text_boxes.get_mut(&tb_id).map(|tb| tb.set_cursor_pos(cursor_pos));
                self.key_mode = KeyboardMode::TextEdit(tb_id, SystemTime::now());
                *cursor = SystemCursor::IBeam;
            }
            HoverItem::HoverLineVertex(shape_id, line_vertex) => {
                self.drag_mode = DragMode::DragLineVertex { shape_id, line_vertex };
                *cursor = SystemCursor::Hand;
            }
            HoverItem::HoverCreateLine { start_pt, last_pt, color } => {
                let id = self.draw_list.add(LineBuilder::new().points2(&start_pt, &last_pt).color(color.0, color.1, color.2).get());
                self.line_select.insert(id, SelectLine::new(start_pt, last_pt));
                self.hover_item = HoverItem::HoverNone;
            }
            HoverItem::HoverNone => {}
        }
    }
    pub fn handle_select(&mut self, pt: &Point, clear_select: bool, cursor: &mut SystemCursor) {
        if clear_select {
            self.clear_selection();
        }
        if let Some(click_shape) = self.draw_list.click_shape(&pt, &self.draw_ctx.viewport) {
            let s = self.draw_list.get(&click_shape).unwrap();
            match s {
                Shape::Polygon(ref draw_poly) => {
                    self.selection.insert(click_shape, self.get_shape_select_box(draw_poly));
                }
                Shape::Line(ref draw_line) => {
                    self.line_select.insert(click_shape, SelectLine(draw_line.clone()));
                }
            };
            self.drag_mode = DragMode::DragShapes { last_pt: *pt, click_shape, clear_select };
            //self.hover_item = HoverItem::HoverRect(click_shape);
            *cursor = SystemCursor::Hand;
        }
        else if clear_select {
            self.drag_mode = DragMode::SelectBox{start_pt: *pt, last_pt: *pt};
        }
    }*/
    /*fn handle_drag(&mut self, pt: &Point, cursor: &mut SystemCursor) {
        let vp = &self.draw_ctx.viewport;
        match self.drag_mode {
            DragMode::DragShapes { ref mut last_pt, ref mut clear_select, .. } => {
                *clear_select = false;
                *cursor = SystemCursor::Hand;
                for (id, rect) in self.selection.iter_mut() {
                    self.draw_list.get_mut(id).map(|s| s.drag(&(*pt - *last_pt)));
                    rect.drag(&(*pt - *last_pt));
                }
                for (id, line) in self.line_select.iter_mut() {
                    self.draw_list.get_mut(id).map(|s| s.drag(&(*pt - *last_pt)));
                    line.drag(&(*pt - *last_pt));
                }
                *last_pt = *pt;
            }
            DragMode::SelectBox {start_pt, ref mut last_pt} => {
                *last_pt = *pt;
                let (shapes, lines)= self.draw_list.get_box_selection(&Rect::new(start_pt, *pt), vp);
                self.selection = HashMap::from_iter(shapes.iter().map(|(id, shape)|
                            (*id, self.get_shape_select_box(shape))
                    ));
                self.line_select = HashMap::from_iter(lines.into_iter().map(|(id, line)| 
                        (id, SelectLine(line.clone()))
                    ));
            }
            DragMode::DragRotate { click_box, ref mut last_angle } => {
                *cursor = SystemCursor::Hand;
                if let Some(sbox) = self.selection.get_mut(&click_box) {
                    let angle = sbox.get_rotate_angle(pt, vp);
                    sbox.0.set_radians(sbox.0.rot + angle - *last_angle);
                    self.draw_list.get_mut(&click_box).map(|s| s.set_rect(&sbox.0.clone()));
                    *last_angle = angle;
                }
            }
            DragMode::DragResize { click_box, ref mut drag_vertex } => {
                *cursor = get_drag_hover_cursor(&drag_vertex);
                if let Some(sbox) = self.selection.get_mut(&click_box) {
                    *drag_vertex = sbox.drag_side(&drag_vertex, &pt, vp);
                    self.draw_list.get_mut(&click_box).map(|s| s.set_rect(&sbox.0.clone()));
               }
               if let Some(tbox) = self.text_boxes.get_mut(&click_box) {
                   let rect = self.draw_list.get(&click_box).unwrap().rect();
                   tbox.format_text(&rect, 0, &self.draw_ctx.render_text);
               }
            }
            DragMode::DragLineVertex { shape_id, line_vertex } => {
                if let Some(sline) = self.line_select.get_mut(&shape_id) {
                  sline.drag_vertex(&line_vertex, &pt);  
                  self.draw_list.get_mut(&shape_id).map(|s| s.drag_vertex(&line_vertex, &pt));
                }
            }
            DragMode::CreateShape { ref mut last_pt, .. } => {
                *last_pt = *pt;
                *cursor = SystemCursor::Crosshair;
            }
            DragMode::DragNone => {}
        }
    }*/
    /*fn handle_hover(&mut self, pt: &Point, cursor: &mut SystemCursor) {
        let mut event_ctx = EventCtx {
            draw_ctx: &self.draw_ctx,
            cursor 
        };
        if let Some((status, cb)) = self.interface.hover(&pt, &mut event_ctx) {
            (cb)(self);
        }
        if let HoverItem::HoverShape(_, ref mut s) = self.hover_item {
            *cursor = SystemCursor::Crosshair;
            match s {
                Shape::Polygon(ref mut poly) => poly.rect.set_center(pt),
                Shape::Line(ref mut draw_line) => {
                    let off = *pt - (draw_line.p1 + draw_line.p2) / 2.;
                    draw_line.p1 += off;
                    draw_line.p2 += off;
                }
            }
        }
        else if let HoverItem::HoverCreateLine { ref mut last_pt, .. } = self.hover_item {
            *last_pt = *pt;
            *cursor = SystemCursor::Crosshair;
        }
        else if let Some((select_id, box_hover)) = self.is_hover_select_box(&pt, vp) {
            match box_hover {
                BoxHover::Rect => { 
                    self.hover_item = HoverItem::HoverRect(select_id);
                    *cursor = SystemCursor::Hand;
                },
                BoxHover::RotateVert => { 
                    self.hover_item = HoverItem::HoverRotate(select_id);
                    *cursor = SystemCursor::Hand;
                },
                BoxHover::Drag(drag_vertex) => {
                    self.hover_item = HoverItem::HoverVertex(select_id, drag_vertex);
                    *cursor = get_drag_hover_cursor(&drag_vertex);
                }
            };
        }
        else if let Some((line_id, line_hover)) = self.is_hover_line(&pt, vp) {
            match line_hover {
                LineHover::Line => self.hover_item = HoverItem::HoverLine(line_id),
                LineHover::Vertex(line_vertex) => self.hover_item = HoverItem::HoverLineVertex(line_id, line_vertex)
            };
            *cursor = SystemCursor::Hand;
        }
        else if let Some((tb_id, cursor_pos)) = self.is_hover_text(pt, vp) {
            *cursor = SystemCursor::IBeam;
            self.hover_item = HoverItem::HoverText(tb_id, cursor_pos);
        }
        else {
            self.hover_item = HoverItem::HoverNone;
        }
    }*/
    /*fn clear_selection(&mut self) {
        self.selection.clear();
        self.line_select.clear();
        self.key_mode = KeyboardMode::KeyboardNone;
    }*/
    pub fn handle_response(&mut self, resp: &Option<WidgetResponse>) {
        if let Some((status, ref cb)) = resp {
            if *status & (WidgetStatus::REDRAW) != WidgetStatus::FINE {
                self.needs_draw = true;
            }
            if *status & (WidgetStatus::REDRAW) != WidgetStatus::FINE {
                self.interface.remeasure(&self.draw_ctx);
            }
            cb(self);
        }
    }
    pub fn handle_mouse_event(&mut self, ev: &Event, _: &Mod) {
        let mut resp: Option<WidgetResponse> = None;
        match *ev {
            Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                if mouse_btn == sdl2::mouse::MouseButton::Left {
                    let pt = Point{x: x as f32,y: y as f32};
                    let mut use_cursor = SystemCursor::Arrow;
                    let mut event_ctx = EventCtx {
                        draw_ctx: &self.draw_ctx,
                        cursor: &mut use_cursor
                    };
                    resp = self.interface.click(&pt, &mut event_ctx);
                    if resp.is_none() {
                        resp = self.interface.deselect();
                    }
                    self.cursors.get(&use_cursor).set();
                }
            } 
            Event::MouseButtonUp{mouse_btn, .. } => {
                if mouse_btn == sdl2::mouse::MouseButton::Left {
                    match self.drag_mode {
                        _ => {}
                    }
                    self.drag_mode = DragMode::DragNone;
                }
            }
            Event::MouseMotion{ x, y, ..} => {
                let pt = Point{x: x as f32,y: y as f32};
                let mut use_cursor = SystemCursor::Arrow;
                let mut event_ctx = EventCtx {
                    draw_ctx: &self.draw_ctx,
                    cursor: &mut use_cursor
                };
                resp = self.interface.hover(&pt, &mut event_ctx);
                self.cursors.get(&use_cursor).set();
            }
            _ => {}
        }
        self.handle_response(&resp);
    }
    /*pub fn get_event_ctx<'a>(&'a self, cursor: &'a mut SystemCursor, _: &Event) -> EventCtx {
        EventCtx {
            draw_ctx: &self.draw_ctx,
            cursor
        }
    }*/
    pub fn handle_keyboard_event(&mut self, ev: &Event) {
        let mut resp: Option<WidgetResponse> = None;
        if let Some(ref key_item) = self.key_item {
            let mut use_cursor = SystemCursor::Arrow;
            if let Event::KeyDown { keycode: Some(keycode), .. } = *ev {
                let mut event_ctx = EventCtx {
                    draw_ctx: &self.draw_ctx,
                    cursor: &mut use_cursor
                };
                resp = key_item.borrow_mut().handle_key_down(&keycode, &mut event_ctx);
            }
        }
        self.handle_response(&resp);
    }
    /*fn delete_selection(&mut self) {
        for id in self.selection.keys() {
            self.draw_list.remove(id); 
            self.text_boxes.remove(id);
        }
        for id in self.line_select.keys() {
            self.draw_list.remove(id); 
        }
        self.line_select.clear();
        self.selection.clear();
    }*/
    fn draw_hover_item(&self) {
        match self.hover_item {
            /*HoverItem::HoverShape(_, ref shape) => {
                shape.draw(&self.draw_ctx);
            }
            HoverItem::HoverCreateLine{start_pt, last_pt, color} => {
                self.draw_ctx.draw_line(start_pt, last_pt, rgb_to_f32(color.0, color.1, color.2), 3.);
            }*/
            _ => {}
        };
    }
    fn draw_drag_item(&self) {
        match self.drag_mode {
            /*DragMode::SelectBox{start_pt, last_pt} => {
                self.draw_ctx.draw_rect(Rect::new(start_pt, last_pt), rgb_to_f32(0, 0, 0), false, Radians(0.));
            }
            DragMode::CreateShape{shape_id, start_pt, last_pt} => {
                let r = Rect::new(start_pt, last_pt);
                shape_id.get_shape(&r, false).draw(&self.draw_ctx);
            }*/
            _ => {}
        }
    }
    pub fn render(&mut self) {
        if self.needs_draw {
            unsafe { gl::Clear(gl::COLOR_BUFFER_BIT); }
            self.interface.draw(&Point::origin(), &self.draw_ctx);
            self.window.gl_swap_window();
            self.needs_draw = false;
        }
    }
}
/* SPEC
* Symbol: AMD
* Strategy:
* Date: 11/26/2019
* Volume: Yes|No
* Gap: Yes|No
* Range: Yes|No
* Level: LEVEL_E Extension
* Pattern: Pennant

Values
Strategy:
   1. MeanReversion Strategy
   2. Trend
   3. LEVEL_E Extension Pivot Re-Entry
   4. LEVEL_D Pivot Entry

Level:
    LEVEL_F
    LEVEL_G
    LEVEL_A
    LEVEL_B
    LEVEL_C
    LEVEL_D
    LEVEL_E
 */
pub fn new_form(ctx: &DrawCtx) -> WidgetList {
    let mut form = WidgetListBuilder::new(Orientation::Vertical, 10, ctx);
    form += label_pair("Symbol: ", Box::new(TextBox::new(ctx.render_text.measure("AMD", 1.))), ctx);
    form += label_pair("Strategy: ", Box::new(DropDown::new(
        vec![
            "MeanReversion Strategy",
            "Trend",
            "LEVEL_E Extension Pivot Re-Entry",
            "LEVEL_D Pivot Entry"
        ], ctx)), ctx);
    form += label_pair("Volume: ", Box::new(DropDown::new(vec![ "Yes", "No"], ctx)), ctx);
    form += label_pair("Gap: ", Box::new(DropDown::new(vec![ "Yes", "No"], ctx)), ctx);
    form += label_pair("Range: ", Box::new(DropDown::new(vec![ "Yes", "No"], ctx)), ctx);
    form += label_pair("Level: ", Box::new(DropDown::new(
        vec![
            "LEVEL_F",
            "LEVEL_G",
            "LEVEL_A",
            "LEVEL_B",
            "LEVEL_C",
            "LEVEL_D",
            "LEVEL_E",
        ], ctx)), ctx);
    form += label_pair("Pattern: ", Box::new(DropDown::new(
        vec![ "Triangle", ], ctx)), ctx);
    let border = Border::new(Point::new(5., 5.), rgb_to_f32(0, 0, 0));
    let submit = Button::new("Submit", TextParams::new(), border.clone(), rgb_to_f32(0, 255, 255), just_status(WidgetStatus::FINE), ctx);
    form += Box::new(submit);
    form.get()
}

pub struct EventCtx<'a> {
    pub draw_ctx: &'a DrawCtx,
    pub cursor: &'a mut SystemCursor
}

#[derive(Copy, Clone, PartialEq)]
enum ClickResponse {
    Clicked,
    NotClicked
}

/*
#[derive(Clone)]
struct SelectLine(DrawLine);


#[derive(Copy, Clone, PartialEq)]
pub enum LineVertex {
    P1, P2
}

#[derive(Copy, Clone, PartialEq)]
pub enum LineHover {
    Line,
    Vertex(LineVertex)
}

impl SelectLine {
    const MIN_VERT_DIST: f32 = 20.;
    fn new(p1: Point, p2: Point) -> Self {
        SelectLine(DrawLine { p1, p2, line_width: 3., color: Point::origin().to_vec4() })
    }
    fn drag(&mut self, off: &Point) {
        self.0.p1 += *off;
        self.0.p2 += *off;
    }
    fn get_hover(&self, pt: &Point, vp: &Point) -> Option<LineHover> {
        if pt.dist(&self.0.p1) <= SelectLine::MIN_VERT_DIST {
            Some(LineHover::Vertex(LineVertex::P1))
        }
        else if pt.dist(&self.0.p2) <= SelectLine::MIN_VERT_DIST {
            Some(LineHover::Vertex(LineVertex::P2))
        }
        else if self.in_bounds(pt, vp) {
            Some(LineHover::Line)
        }
        else { None }
    }
    fn drag_vertex(&mut self, vtx: &LineVertex, pt: &Point) {
        self.0.drag_vertex(vtx, pt);
    }
    fn draw_verts(&self, draw_ctx: &DrawCtx) {
        let radi = 7.;
        &[self.0.p1, self.0.p2].iter().
            for_each(|v| draw_ctx.draw_circle(radi, *v, rgb_to_f32(255, 255, 255), false));
    }
    fn draw(&self, draw_ctx: &DrawCtx) {
        self.draw_verts(draw_ctx);
    }
}

impl InBounds for SelectLine {
    fn in_bounds(&self, p: &Point, vp: &Point) -> bool {
        self.0.in_bounds(p, vp)
    }
}

#[derive(Clone)]
struct ShapeSelectBox(RotateRect);

pub enum BoxHover {
    RotateVert,
    Rect,
    Drag(DragVertex),
}

#[derive(PartialEq, Debug, Copy, Clone, FromPrimitive)]
pub enum DragVertex {
    TopLeft = 0,
    TopRight = 1,
    BottomRight = 2,
    BottomLeft = 3,
    TopCenter = 4,
    Right = 5,
    BottomCenter = 6,
    Left = 7,
}

fn get_drag_hover_cursor(drag_vertex: &DragVertex) -> SystemCursor {
    match drag_vertex {
        DragVertex::Left | DragVertex::Right => {
            SystemCursor::SizeWE
        }
        DragVertex::TopCenter | DragVertex::BottomCenter => {
            SystemCursor::SizeNS
        }
        DragVertex::TopLeft | DragVertex::BottomRight => {
            SystemCursor::SizeNWSE
        }
        DragVertex::TopRight | DragVertex::BottomLeft => {
            SystemCursor::SizeNESW
        }
    }
}

impl ShapeSelectBox {
    const MIN_CORNER_DIST: u32 = 10;

    fn drag(&mut self, off: &Point) {
        self.0.drag(off);
    }

    fn drag_side_swap_vertex(&mut self, vertex1: &DragVertex, vertex2: &DragVertex, start: &mut f32, min_max: &mut f32, new: &f32) 
        -> DragVertex
    {
        if *start < *min_max {
            *start = *new;
            if *new > *min_max {
                std::mem::swap(start, min_max);
                *vertex2
            }
            else {
                *vertex1
            }
        }
        else {
            *start = *new;
            if *new < *min_max {
                std::mem::swap(start, min_max);
                *vertex2
            }
            else {
                *vertex1
            }
        }
    }
    #[allow(dead_code)]
    fn drag_corner_swap_vertex(&mut self, vertex1: &DragVertex, vertex2: &DragVertex, start: &mut Point, min_max: &mut Point, new: &Point)
        -> DragVertex
    {
        let mut pt = *new;
        let width = f32::abs(start.x - min_max.x);
        let height = f32::abs(start.y - min_max.y);
        //if y is shrinking, or both shrinking or both expanding, base on x
        if  ((new.y <= start.y) != (start.y <= min_max.y) ||
            (new.x <= start.x) == (start.x <= min_max.x) && (new.y <= start.y) == (start.y <= min_max.y))
            && (f32::abs(new.x - start.x) < ShapeSelectBox::MIN_CORNER_DIST as f32)
        {
            pt.y = start.y + (new.x - start.x) * height / width;
        }
        //x shrinking, base on y
        else {
            pt.x = start.x + (new.y - start.y) * width / height;
        }
        self.drag_side_swap_vertex(vertex1, vertex2, &mut start.x, &mut min_max.x, &pt.x);
        self.drag_side_swap_vertex(vertex1, vertex2, &mut start.y, &mut min_max.y, &pt.y)
    }
    fn drag_side(&mut self, drag_vertex: &DragVertex, new_pt: &Point, vp: &Point) -> DragVertex {
        let trans = RectTransform::new(&self.0, vp);
        let model_pt: Point = trans.pixel_to_model(new_pt).into();
        let mut r = Rect::default(); 
        let new_vtx = match *drag_vertex {
            DragVertex::TopCenter => {
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::BottomCenter, &mut r.c1.y, &mut r.c2.y, &model_pt.y)
            }
            DragVertex::BottomCenter => {
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::TopCenter, &mut r.c2.y, &mut r.c1.y, &model_pt.y)
            }
            DragVertex::Left => {
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::Right, &mut r.c1.x, &mut r.c2.x, &model_pt.x)
            }
            DragVertex::Right => {
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::Left, &mut r.c2.x, &mut r.c1.x, &model_pt.x)
            }
            DragVertex::TopLeft => {
                let h = (model_pt.x - r.c1.x) * r.height() / r.width();
                let pt = Point{x: model_pt.x, y: r.c1.y + h};
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::BottomRight, &mut r.c1.x, &mut r.c2.x, &pt.x);
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::BottomRight, &mut r.c1.y, &mut r.c2.y, &pt.y)
            }
            DragVertex::BottomRight => {
                let h = (model_pt.x - r.c2.x) * r.height() / r.width();
                let pt = Point{x: model_pt.x, y: r.c2.y + h};
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::TopLeft, &mut r.c2.x, &mut r.c1.x, &pt.x);
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::TopLeft, &mut r.c2.y, &mut r.c1.y, &pt.y)
            }
            DragVertex::TopRight => {
                let h = (r.c2.x - model_pt.x) * r.height() / r.width();
                let pt = Point{x: model_pt.x, y: r.c1.y + h};
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::BottomLeft, &mut r.c2.x, &mut r.c1.x, &pt.x);
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::BottomLeft, &mut r.c1.y, &mut r.c2.y, &pt.y)
            }
            DragVertex::BottomLeft => {
                let h = (r.c1.x - model_pt.x) * r.height() / r.width();
                let pt = Point{x: model_pt.x, y: r.c2.y + h};
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::TopRight, &mut r.c1.x, &mut r.c2.x, &pt.x);
                self.drag_side_swap_vertex(drag_vertex, &DragVertex::TopRight, &mut r.c2.y, &mut r.c1.y, &pt.y)
            }
        };
        self.0.resize(&r, vp);
        new_vtx
    }

    fn get_drag_vertex(&self, pt: &Point, vp: &Point) -> Option<DragVertex> {
        self.get_drag_points(vp).iter().enumerate()
            .find(|(_, p)| (p.dist(&pt) as u32) < ShapeSelectBox::MIN_CORNER_DIST)
            .map(|(i, _)| FromPrimitive::from_usize(i).unwrap())
    }

    #[inline]
    fn get_drag_points(&self, vp: &Point) -> Vec<Point> {
        let mut points = self.0.verts(vp);
        points.push((points[0] + points[1]) / 2.);
        points.push((points[1] + points[2]) / 2.);
        points.push((points[2] + points[3]) / 2.);
        points.push((points[3] + points[0]) / 2.);
        points
    }
    fn get_rotate_points(&self, vp: &Point) -> Vec<Point> {
        let mut r = self.0.clone();
        r.size *= Point::new(1.2,1.2);
        r.set_center(&self.0.center(vp));
        r.verts(vp)
    }
    fn is_hover_rotate(&self, pt: &Point, vp: &Point) -> bool {
        let radi = 12.;
        self.get_rotate_points(vp).iter().any(|p| p.dist(&pt) < radi)
    }
    fn get_rotate_angle(&self, pt: &Point, vp: &Point) -> Radians {
        let center = self.0.center(vp);
        let dist = *pt - center;
        let angle = dist.y.atan2(dist.x);
        Radians(angle)
    }
    fn draw_drag_circles(&self, draw_ctx: &DrawCtx) {
        let radi = 5.;
        self.get_drag_points(&draw_ctx.viewport).iter()
            .for_each(|v| draw_ctx.draw_circle(radi, *v, rgb_to_f32(255, 255, 255), true));
    } 
    fn draw_rotate_circles(&self, draw_ctx: &DrawCtx) {
        let radi = 5.;
        self.get_rotate_points(&draw_ctx.viewport).iter()
            .for_each(|v| draw_ctx.draw_circle(radi, *v, rgb_to_f32(0, 0, 255), false));
    }
    fn draw(&self, draw_ctx: &DrawCtx) {
        //draw box
        self.0.builder().color(255,255,255).fill(false).get().draw(draw_ctx);
        self.draw_drag_circles(draw_ctx);
        self.draw_rotate_circles(draw_ctx);
    }
    fn get_hover(&self, p: &Point, vp: &Point) -> Option<BoxHover> {
        if self.0.in_bounds(p,vp) {
            Some(BoxHover::Rect)
        }
        else if let Some(v) = self.get_drag_vertex(p, vp) {
            Some(BoxHover::Drag(v))
        }
        else if self.is_hover_rotate(p, vp) {
            Some(BoxHover::RotateVert)
        }
        else { None }
    }
}

impl InBounds for ShapeSelectBox {
    fn in_bounds(&self, p: &Point, vp: &Point) -> bool {
        self.0.in_bounds(p, vp)
    }
}



#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ShapeBarShape {
    Circle = 0,
    Triangle = 1,
    Rect = 2,
    TextBox = 3,
    Line = 4,
    ColorPicker = 5,
}

pub struct ShapeBarItem {
    id: ShapeBarShape,
    cache: ShapeCache
}

impl ShapeBarItem {
    fn new(id: ShapeBarShape, shape: Shape) -> Self {
        ShapeBarItem { id, cache: ShapeCache::new(shape) }
    }
}

impl ShapeBarShape {
    const DEFAULT_SIZE: f32 = 30.;
    const DEFAULT_COLOR: (u8, u8, u8) = (255, 0, 0);
    fn prim_type(&self) -> PrimType {
        match self {
            ShapeBarShape::Circle => PrimType::Circle,
            ShapeBarShape::Triangle => PrimType::Triangle,
            ShapeBarShape::Rect | ShapeBarShape::TextBox => PrimType::Rect,
            ShapeBarShape::Line => PrimType::Line,
            _ => PrimType::Rect
        }
    }
    fn get_shape(&self, r: &Rect, fill: bool) -> Shape {
        let color = 
            match self {
                ShapeBarShape::TextBox => (255, 255, 255),
                _ => ShapeBarShape::DEFAULT_COLOR
            };
        match self {
            ShapeBarShape::Line => {
                Shape::Line(DrawLine { p1: r.left_center(), p2: r.right_center(), line_width: 3., 
                    color: rgb_to_f32(color.0, color.1, color.2)})
            },
            _ => {
                let rect = RotateRect::new(r.c1, r.size(), Radians(0.));
                let ptype = self.prim_type();
                let prim = if !fill && ptype == PrimType::Circle { PrimType::Ring } else { ptype };
                Shape::Polygon(
                    DrawPolygon { rect, fill, prim, color: rgb_to_f32(color.0, color.1, color.2) })
            }
        }
    }
}

impl Widget for ShapeBarItem {
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        self.cache.draw(*offset, ctx);
    }
    fn measure(&self, _: &DrawCtx) -> Point {
        Point::new(ShapeBarShape::DEFAULT_SIZE, ShapeBarShape::DEFAULT_SIZE)
    }
    fn click(&mut self, _: &Point, ev_ctx: &mut EventCtx) -> Option<WidgetResponse> {
        /*ev_ctx.cursor = SystemCursor::Crosshair;
        let id = self.id;
        let mut s = self.cache.get();
        s.set_fill(false);
        Some(Rc::new(move |app: &mut AppState| {
            app.hover_item = HoverItem::HoverShape(id, s.clone());
        }))*/
        None
    }
}

struct ShapeBar {
    widget_list: WidgetList,
}

impl ShapeBar {
    fn new(ctx: &DrawCtx) -> Self {
        let shape_bar_shapes = [ShapeBarShape::Circle, ShapeBarShape::Triangle, 
                                ShapeBarShape::Rect, ShapeBarShape::TextBox, ShapeBarShape::Line];
        let rect_size = Point::new(ShapeBarShape::DEFAULT_SIZE, ShapeBarShape::DEFAULT_SIZE);
        let mut widget_list = WidgetList::new(Orientation::Vertical, 20);
        for sb in shape_bar_shapes.iter() {
            let fill = *sb != ShapeBarShape::TextBox;
            let rect = Rect::new(Point::origin(), rect_size);
            let item = ShapeBarItem::new(*sb, sb.get_shape(&rect, fill));
            widget_list.add(Box::new(item), ctx);
        }
        ShapeBar { 
            widget_list,
        }
    }
}
impl Widget for ShapeBar {
    fn measure(&self, ctx: &DrawCtx) -> Point {
        self.widget_list.measure(ctx)
    }
    fn click(&mut self, off_pt: &Point, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.widget_list.click(off_pt, ctx)
    }
    fn draw(&self, offset: &Point, ctx: &DrawCtx) {
        if let DrawMode::Canvas = ctx.draw_mode {
            let r = Rect { c1: *offset, c2: *offset + self.widget_list.size };
            ctx.draw_rect(r, rgb_to_f32(120, 50, 200), true, Radians(0.)); 
            self.widget_list.draw(offset, ctx)
        }
    }
}*/



/*
pub fn new_tab_bar(ctx: &DrawCtx) -> WidgetList { 
    let border = Border::new(Point::new(5., 5.), rgb_to_f32(0, 0, 0));
    let canvas_button = Button::new(
        "Canvas",
        TextParams::new(),
        border.clone(),
        rgb_to_f32(0, 255, 255),
        just_cb(Rc::new(|_: &mut AppState| { 
            //app.app_mode = AppMode::Canvas;
            //app.draw_ctx.draw_mode = DrawMode::Canvas;
         })),
        ctx
    );
    let graph_button = Button::new(
        "Graph",
        TextParams::new(),
        border.clone(),
        rgb_to_f32(0, 255, 255),
        just_cb(Rc::new(|_: &mut AppState| { 
            //app.app_mode = AppMode::Graph(GraphModeState::new());
            //app.draw_ctx.draw_mode = DrawMode::Graph;
        })),
        ctx
    );
    let mut wl = WidgetList::new(Orientation::Horizontal, 5);
    wl.add(Box::new(canvas_button), ctx);
    wl.add(Box::new(graph_button), ctx);
    wl
}*/
