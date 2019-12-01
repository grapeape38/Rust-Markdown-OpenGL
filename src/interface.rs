extern crate sdl2;
extern crate chrono;

use std::collections::{HashMap};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::{Cursor, SystemCursor};
use sdl2::video::Window;
use std::fs::File;
use std::io::{Write};
use crate::primitives::*;
use crate::render_text::{TextParams};
use std::rc::Rc;
use std::cell::RefCell;
use crate::widgets::*;
use chrono::Datelike;

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
}

pub struct AppState {
    pub interface: WidgetGrid,
    pub key_item: Option<HandleKeyItem>,
    pub draw_ctx: DrawCtx,
    window: Window,
    cursors: CursorMap,
    needs_draw: bool,
}

pub type CallbackFn = Rc<dyn Fn(&mut AppState)>;

pub trait HandleKey {
    fn handle_key_down(&mut self, kc: &Keycode, rt: &EventCtx) -> Option<WidgetResponse>;
}

pub type HandleKeyItem = Rc<RefCell<dyn HandleKey>>;

impl AppState {
    pub fn new(viewport: &Point, window: Window) -> AppState {
        let draw_ctx = DrawCtx::new(viewport);
        let interface = new_form(&draw_ctx);
        AppState {
            draw_ctx,
            interface,
            window,
            key_item: None, 
            cursors: CursorMap::new(),
            needs_draw: true
        }
    }
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
    pub fn render(&mut self) {
        if self.needs_draw {
            unsafe { gl::Clear(gl::COLOR_BUFFER_BIT); }
            self.interface.draw(&Point::origin(), &self.draw_ctx);
            self.window.gl_swap_window();
            self.needs_draw = false;
        }
    }
    /*
    title: "AMD - Trend Extension"
    date: 2019-11-26T01:00:00-05:00
    draft: false*/
    pub fn serialize(&self) {
        let mut md = MDDoc::empty();
        self.interface.serialize(&mut md);
        let path = format!("/opt/blocktradingsystems/tradelog/content/holdings/{}/{}.md", 
                           md.portfolio,
                           md.title.symbol);
        println!("{}", path);
        let date = chrono::Local::now();
        let title = format!("---\ntitle: \"{} - {}\"\ndate: {}\ndraft: false\n---\n# Entry\n", md.title.symbol, md.title.strategy, date.to_rfc3339());
        let addenda = format!("\n# Log\n* {}/{}/{}", 
                              date.month(), date.day(), date.year());
        let write_file = || {
            let mut file = File::create(&path)?;
            file.write(&title.as_bytes())?;
            file.write(&md.body)?;
            file.write(&addenda.as_bytes())
        };
        match write_file() {
            Ok(_) => { println!("Wrote to file {}", path); }
            Err(e) => { println!("Error writing to file {:?}", e); }
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

pub fn new_form(ctx: &DrawCtx) -> WidgetGrid {
    let mut form = WidgetGrid::new(Point::new(10., 10.)).builder(ctx);
    form += vec![new_label("Symbol:"), new_serialize::<SymbolSerializer>(new_textbox(6, "", ctx))];
    form += vec![new_label("Strategy:"), new_serialize::<StrategySerializer>(new_dropdown( 
        vec![
            "Mean Reversion Strategy",
            "Trend",
            "LEVEL_E Extension Pivot Re-Entry",
            "LEVEL_D Pivot Entry"
        ], 0, ctx))];
    form += vec![new_label("Date:"), Box::new(DateWidget::new(ctx))];
    form += vec![new_label("Volume:"), new_dropdown(vec![ "Yes", "No"], 0, ctx)];
    form += vec![new_label("Gap:"), new_dropdown(vec![ "Yes", "No"], 0, ctx)];
    form += vec![new_label("Range:"), new_dropdown(vec![ "Yes", "No"], 0, ctx)];
    form += vec![new_label("Level:"), new_h_list(vec![new_dropdown(
        vec![
            "LEVEL_F",
            "LEVEL_G",
            "LEVEL_A",
            "LEVEL_B",
            "LEVEL_C",
            "LEVEL_D",
            "LEVEL_E",
        ], 0, ctx), new_dropdown(vec!["Plus", "Minus"], 0, ctx)], 10, ctx)];
    form += vec![new_label("Pattern:"), new_dropdown( 
        vec![ "Triangle", ], 0, ctx)];
    form += vec![
        new_label("Portfolio:"), 
        new_serialize::<PortfolioSerializer>(new_dropdown(vec![ "A", "B"], 0, ctx))];
    let border = Border::new(Point::new(5., 5.), rgb_to_f32(0, 0, 0));
    let mut submit = Button::new(border, rgb_to_f32(0, 255, 255), 
        just_cb(Rc::new(|app: &mut AppState| app.serialize()))
    ).builder(ctx);
    submit += Label::new("Submit", None, None, None, TextParams::new());
    form += vec![new_serialize::<SkipSerializer>(submit.widget())];
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
