extern crate chrono;
extern crate sdl2;

use crate::primitives::*;
use crate::render_text::TextParams;
use crate::widgets::*;
//use chrono::Datelike;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::{Cursor, SystemCursor};
use sdl2::video::Window;
use std::cell::RefCell;
use std::collections::HashMap;
//use std::fs::File;
//use std::io::Write;
use std::rc::Rc;

pub struct CursorMap(HashMap<SystemCursor, Cursor>);
impl CursorMap {
    fn new() -> Self {
        let mut m = HashMap::new();
        m.insert(
            SystemCursor::Arrow,
            Cursor::from_system(SystemCursor::Arrow).unwrap(),
        );
        m.insert(
            SystemCursor::Hand,
            Cursor::from_system(SystemCursor::Hand).unwrap(),
        );
        m.insert(
            SystemCursor::Crosshair,
            Cursor::from_system(SystemCursor::Crosshair).unwrap(),
        );
        m.insert(
            SystemCursor::SizeNESW,
            Cursor::from_system(SystemCursor::SizeNESW).unwrap(),
        );
        m.insert(
            SystemCursor::SizeNS,
            Cursor::from_system(SystemCursor::SizeNS).unwrap(),
        );
        m.insert(
            SystemCursor::SizeNWSE,
            Cursor::from_system(SystemCursor::SizeNWSE).unwrap(),
        );
        m.insert(
            SystemCursor::SizeWE,
            Cursor::from_system(SystemCursor::SizeWE).unwrap(),
        );
        m.insert(
            SystemCursor::IBeam,
            Cursor::from_system(SystemCursor::IBeam).unwrap(),
        );
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
            true => ClickResponse::Clicked,
            false => ClickResponse::NotClicked,
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
            Shape::Line(_) => {}
        }
    }
}

pub struct AppState {
    pub interface: WidgetS,
    //pub key_item: Option<HandleKeyItem>,
    pub select_state: SelectionState,
    pub draw_ctx: DrawCtx,
    window: Window,
    cursors: CursorMap,
    needs_draw: bool,
}

pub type CallbackFn = Rc<dyn Fn(&mut AppState)>;

pub trait HandleKey {
    fn handle_key_down(&mut self, kc: &Keycode, rt: &mut EventCtx);
}

pub type HandleKeyItem = Rc<RefCell<dyn HandleKey>>;

const INTERFACE_OFFSET: (f32, f32) = (15., 15.);

impl AppState {
    pub fn new(viewport: &Point, window: Window) -> AppState {
        let draw_ctx = DrawCtx::new(viewport);
        let interface = new_form(&draw_ctx);
        let select_state = SelectionState::new(&interface);
        select_state.print();
        AppState {
            draw_ctx,
            interface,
            window,
            select_state,
            cursors: CursorMap::new(),
            needs_draw: true,
        }
    }
    pub fn handle_response(&mut self, status: WidgetStatus, callbacks: Vec<CallbackFn>) {
        if status & (WidgetStatus::REDRAW) != WidgetStatus::FINE {
            self.needs_draw = true;
        }
        if status & (WidgetStatus::REDRAW) != WidgetStatus::FINE {
            self.interface.remeasure(&self.draw_ctx);
        }
        for cb in callbacks {
            cb(self);
        }
    }
    /*pub fn new_widget_ctx<'a>(&'a self, use_cursor: &'a mut SystemCursor) -> WidgetEventCtx<'a> {
        WidgetEventCtx { 
            draw_ctx: &self.draw_ctx,
            cursor: use_cursor,
            select_ctx: SelectCtx {
                state: &self.select_state,
                select_pos: 0
            },
            widget_idx: 0,
        }
    }*/
    pub fn handle_mouse_event(&mut self, ev: &Event, _: &Mod) {
        let mut use_cursor = SystemCursor::Arrow;
        /*let mut event_ctx = EventCtx {
            draw_ctx: &self.draw_ctx,
            cursor: &mut use_cursor,
        };*/
        let mut widget_ctx = WidgetEventCtx::new(&self.draw_ctx, &mut use_cursor, &self.select_state);
        let _ = match *ev {
            Event::MouseButtonDown {
                mouse_btn, x, y, ..
            } => {
                if mouse_btn == sdl2::mouse::MouseButton::Left {
                    let pt = Point {
                        x: x as f32 - INTERFACE_OFFSET.0,
                        y: y as f32 - INTERFACE_OFFSET.1,
                    };
                    self.interface.click(&pt, &mut widget_ctx)
                        //.or(self.select_state.set_select(None, &mut event_ctx))
                }
                else { None }
            }
            Event::MouseButtonUp { mouse_btn, .. } => {
                if mouse_btn == sdl2::mouse::MouseButton::Left {}
                None
            }
            Event::MouseMotion { x, y, .. } => {
                let pt = Point {
                    x: x as f32,
                    y: y as f32,
                };
                self.interface.hover(&pt, &mut widget_ctx)
            }
            _ => { None }
        };
        self.handle_response(widget_ctx.status, widget_ctx.callbacks);
        self.cursors.get(&use_cursor).set();
    }
    pub fn handle_keyboard_event(&mut self, ev: &Event) {
        let mut use_cursor = SystemCursor::Arrow;
        let mut event_ctx = EventCtx {
            draw_ctx: &self.draw_ctx,
            cursor: &mut use_cursor,
            callbacks: Vec::new(),
            status: WidgetStatus::FINE
        };
        if let Event::KeyDown {
            keycode: Some(keycode),
            ..
        } = *ev {
            if self.select_state.is_select() {
                let resp = self.select_state.handle_key_down(&keycode, &mut event_ctx);
                if resp.is_none() {
                    if let Keycode::Tab = keycode {
                        self.select_state.select_next(&mut event_ctx);
                    }
                }
            }
        }
        self.handle_response(event_ctx.status, event_ctx.callbacks);
    }
    pub fn set_select(&mut self, select_idx: Option<usize>) {
        let mut use_cursor = SystemCursor::Arrow;
        let mut event_ctx = EventCtx {
            draw_ctx: &self.draw_ctx,
            cursor: &mut use_cursor,
            callbacks: Vec::new(),
            status: WidgetStatus::FINE
        };
        self.select_state.set_select(select_idx, &mut event_ctx);
        self.handle_response(event_ctx.status, event_ctx.callbacks);
    }
    pub fn render(&mut self) {
        if self.needs_draw {
            unsafe {
                gl::Clear(gl::COLOR_BUFFER_BIT);
            }
            let mut widget_ctx = WidgetDrawCtx::new(&self.draw_ctx, &self.select_state);
            self.interface.draw(
                &Point::new(INTERFACE_OFFSET.0, INTERFACE_OFFSET.1),
                &mut widget_ctx,
            );
            self.window.gl_swap_window();
            self.needs_draw = false;
        }
    }
    /*
    title: "AMD - Trend Extension"
    date: 2019-11-26T01:00:00-05:00
    draft: false*/
    /*pub fn serialize(&self) {
        let mut md = MDDoc::empty();
        self.interface.serialize(&mut md);
        let path = format!(
            "/opt/blocktradingsystems/tradelog/content/holdings/{}/{}.md",
            md.portfolio, md.title.symbol
        );
        println!("{}", path);
        let date = chrono::Local::now();
        let title = format!(
            "---\ntitle: \"{} - {}\"\ndate: {}\ndraft: false\n---\n# Entry\n",
            md.title.symbol,
            md.title.strategy,
            date.to_rfc3339()
        );
        let addenda = format!("\n# Log\n* {}/{}/{}", date.month(), date.day(), date.year());
        let write_file = || {
            let mut file = File::create(&path)?;
            file.write(&title.as_bytes())?;
            file.write(&md.body)?;
            file.write(&addenda.as_bytes())
        };
        match write_file() {
            Ok(_) => {
                println!("Wrote to file {}", path);
            }
            Err(e) => {
                println!("Error writing to file {:?}", e);
            }
        }
    }*/
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

pub fn new_form(ctx: &DrawCtx) -> WidgetS {//WidgetGrid {
    let mut form = new_container(WidgetGrid::new(2, Point::new(10., 10.)));
    form += vec![new_label("Symbol:"), new_textbox("", 6)];
    form += vec![new_label("Strategy:"), new_dropdown(vec!["Trend", "Mean Reversion"], 0)];
    form += vec![
        new_label("Volume:"),
        new_dropdown(vec!["Yes", "No"], 0),
    ];
    form += vec![new_label("Gap:"), new_dropdown(vec!["Yes", "No"], 0)];
    form += vec![new_label("Range:"), new_dropdown(vec!["Yes", "No"], 0)];
    form += vec![
        new_label("Level:"),
        new_container(WidgetList::new(Orientation::Horizontal, 10)) +
            vec![
                new_dropdown(
                    vec![
                        "LEVEL_C", "LEVEL_A", "LEVEL_D", "LEVEL_B", "LEVEL_E", "LEVEL_F", "LEVEL_G",
                    ],
                    0,
                ),
                new_dropdown(vec![" ", "Minus"], 0),
            ],
    ];
    form += vec![new_label("Pattern:"), new_textbox("", 30)];
    form += vec![
        new_label("Portfolio:"),
        new_dropdown(vec!["A", "B"], 0),
    ];
    let border = Border::new(Point::new(5., 5.), rgb_to_f32(0, 0, 0));
    let mut submit = new_button(
        border,
        rgb_to_f32(0, 255, 255),
        Rc::new(|_: &mut AppState| {}));
    submit += new_label("Submit");
    form += submit;
    form.remeasure(ctx);
    form
}

pub struct EventCtx<'a> {
    pub draw_ctx: &'a DrawCtx,
    pub cursor: &'a mut SystemCursor,
    pub callbacks: Vec<CallbackFn>,
    pub status: WidgetStatus
}

impl<'a> EventCtx<'a> {
    pub fn push_cb(&mut self, cb: CallbackFn) {
        self.callbacks.push(cb);
    }
    pub fn set_redraw(&mut self) {
        self.status |= WidgetStatus::REDRAW;
    }
    pub fn set_remeasure(&mut self) {
        self.status |= WidgetStatus::REMEASURE;
    }
}

#[derive(Copy, Clone, PartialEq)]
enum ClickResponse {
    Clicked,
    NotClicked,
}
