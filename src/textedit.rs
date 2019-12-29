extern crate ropey;
extern crate sdl2;

use crate::interface::{AppState, EventCtx, HandleKey};
use crate::primitives::{rgb_to_f32, DrawCtx, Point, Radians, Rect, RotateRect};
use crate::render_text::{RenderText, TextParams};
use crate::widgets::{
    just_cb, just_status, MDDoc, SelectionT, Widget, 
    WidgetDrawCtx, WidgetResponse, WidgetStatus, WidgetEventCtx
};
use ropey::Rope;
use sdl2::keyboard::Keycode;
use sdl2::mouse::SystemCursor;
use std::time::SystemTime;
use std::rc::Rc;

#[derive(Debug)]
struct TextCursor {
    char_idx: usize,
}

impl TextCursor {
    fn new() -> Self {
        TextCursor { char_idx: 0 }
    }
}

pub enum TextCursorDirection {
    Up,
    Down,
    Left,
    Right,
}

//#[derive(Debug)]
pub struct TextEdit {
    text_rope: Rope,
    top_line: usize,
    size: Point,
    cursor: TextCursor,
    select_time: Option<SystemTime>,
    text_params: TextParams,
}

impl TextEdit {
    fn new(text: &str, size: Point) -> Self {
        TextEdit {
            text_rope: Rope::from_str(text),
            top_line: 0,
            size,
            select_time: None,
            cursor: TextCursor::new(),
            text_params: TextParams::new(),
        }
    }
    pub fn insert_char(&mut self, ch: char, rt: &RenderText) {
        let scale = self.text_params.scale;
        let cursor_line = self.text_rope.char_to_line(self.cursor.char_idx);
        let line = self.text_rope.line(cursor_line);
        let new_width = rt.char_size(ch, scale).x + rt.measure(line.as_str().unwrap(), scale).x;
        if new_width > self.size.x
            && (1 + self.text_rope.len_lines()) as f32 * rt.line_height(scale) > self.size.y
        {
            return;
        }
        let cursor_pos = self.cursor.char_idx - self.text_rope.line_to_char(cursor_line);
        let insert_break = cursor_pos == line.len_chars()
            && new_width > self.size.x
            && self.cursor.char_idx == self.text_rope.len_chars();
        self.text_rope.insert_char(self.cursor.char_idx, ch);
        self.cursor.char_idx += 1;
        if insert_break {
            self.text_rope.insert_char(self.cursor.char_idx - 1, '\n');
            self.cursor.char_idx += 1;
        }
        if self.cursor.char_idx < self.text_rope.len_chars() {
            self.format_text(cursor_line, rt);
        }
    }
    pub fn delete_char(&mut self, rt: &RenderText) {
        if self.text_rope.len_chars() == 0 {
            return;
        }
        let cursor_line = self.text_rope.char_to_line(self.cursor.char_idx);
        if cursor_line > 0 && self.cursor.char_idx == self.text_rope.line_to_char(cursor_line) {
            self.cursor.char_idx -= 1;
        }
        //println!("Cursor char index: {:?}, line char index: {:?}", self.cursor.char_idx, self.text_rope.line_to_char(cursor_line));
        self.text_rope
            .remove(self.cursor.char_idx - 1..self.cursor.char_idx);
        self.cursor.char_idx -= 1;
        if self.text_rope.len_chars() > 0 && self.cursor.char_idx < self.text_rope.len_chars() - 1 {
            self.format_text(cursor_line, rt);
        }
    }
    pub fn hover_text(&self, pt: &Point, rect: &RotateRect, ctx: &DrawCtx) -> Option<usize> {
        let mut pt2 = rect.transform(&ctx.viewport).pixel_to_model(pt);
        pt2.x *= rect.size.x;
        pt2.y *= rect.size.y;
        let rt = &ctx.render_text;
        let n_line = (pt2.y / rt.line_height(self.text_params.scale)) as i32;
        if pt2.x < 0.
            || pt2.x > rect.size.x
            || n_line < 0
            || n_line >= self.text_rope.len_lines() as i32
        {
            return None;
        }
        let line_idx = n_line as usize;
        let start_char = self.text_rope.line_to_char(line_idx);
        let end_char = self.text_rope.line_to_char(line_idx + 1);
        //println!("Hover Text! Line index: {:?} Start char pos: {:?}, End char pos {:?}", line_idx, start_char, end_char);
        let mut line_x = 0.;
        (start_char + 1..end_char)
            .take_while(|i| {
                line_x += rt
                    .char_size_w_advance(self.text_rope.char(i - 1), self.text_params.scale)
                    .x;
                line_x <= pt2.x
            })
            .last()
    }
    pub fn set_cursor_pos(&mut self, cursor_idx: usize) {
        self.cursor.char_idx =
            std::cmp::max(0, std::cmp::min(self.text_rope.len_chars(), cursor_idx));
    }
    pub fn move_cursor(&mut self, dir: TextCursorDirection) {
        let cursor_line = self.text_rope.char_to_line(self.cursor.char_idx);
        let line = self.text_rope.line(cursor_line);
        let cursor_pos = self.cursor.char_idx - self.text_rope.line_to_char(cursor_line);
        //println!("Cursor line: {:?} Cursor Pos: {:?}", line, cursor_pos);
        match dir {
            TextCursorDirection::Left => {
                if self.cursor.char_idx > 0 {
                    if cursor_pos == 0 {
                        self.cursor.char_idx -= 1;
                    }
                    self.cursor.char_idx -= 1;
                }
            }
            TextCursorDirection::Right => {
                if self.cursor.char_idx < self.text_rope.len_chars() {
                    if cursor_pos == line.len_chars() {
                        self.cursor.char_idx += 1;
                    }
                    self.cursor.char_idx += 1;
                }
            }
            TextCursorDirection::Up => {
                if cursor_line > 0 {
                    let prev_line_char = self.text_rope.line_to_char(cursor_line - 1);
                    let prev_line = self.text_rope.line(cursor_line - 1);
                    self.cursor.char_idx = std::cmp::min(
                        prev_line_char + prev_line.len_chars(),
                        prev_line_char + cursor_pos,
                    );
                }
            }
            TextCursorDirection::Down => {
                if cursor_line < self.text_rope.len_lines() - 1 {
                    let next_line_char = self.text_rope.line_to_char(cursor_line + 1);
                    let next_line = self.text_rope.line(cursor_line + 1);
                    self.cursor.char_idx = std::cmp::min(
                        next_line_char + next_line.len_chars() - 1,
                        next_line_char + cursor_pos,
                    );
                }
            }
        }
    }
    pub fn needs_format(&self, start_line: usize, rt: &RenderText) -> bool {
        let start_line_width = rt
            .measure(
                self.text_rope.line(start_line).as_str().unwrap(),
                self.text_params.scale,
            )
            .x;
        if start_line_width > self.size.x {
            return true;
        }
        if start_line < self.text_rope.len_lines() - 1 {
            let next_line = self.text_rope.line(start_line + 1);
            if next_line.len_chars() > 0 {
                let next_line_char = next_line.char(0);
                let next_line_char_width = rt.char_size(next_line_char, self.text_params.scale).x;
                return start_line_width + next_line_char_width <= self.size.x;
            }
        }
        false
    }
    pub fn format_text(&mut self, start_line: usize, rt: &RenderText) {
        if !self.needs_format(start_line, rt) {
            return;
        }
        let start_char = self.text_rope.line_to_char(start_line);
        let mut line_x = 0.;
        let mut line_breaks = Vec::new();
        for (i, c) in self.text_rope.slice(start_char..).chars().enumerate() {
            let add_break = line_x + rt.char_size(c, self.text_params.scale).x > self.size.x;
            let was_break = c == '\n';
            if add_break {
                line_x = 0.;
            }
            if add_break != was_break {
                line_breaks.push((i + start_char, add_break));
            }
            line_x += rt.char_size_w_advance(c, self.text_params.scale).x;
        }
        let mut offset: i32 = 0;
        for (idx, is_add) in line_breaks {
            let uidx = (idx as i32 + offset) as usize;
            if is_add {
                self.text_rope.insert_char(uidx, '\n');
                if uidx < self.cursor.char_idx {
                    self.cursor.char_idx += 1;
                }
                offset += 1;
            } else {
                self.text_rope.remove(uidx..uidx + 1);
                if uidx < self.cursor.char_idx {
                    self.cursor.char_idx -= 1;
                }
                offset -= 1;
            }
        }
    }
    pub fn draw(&self, rect: &RotateRect, draw_ctx: &DrawCtx) {
        let cursor_line = self.text_rope.char_to_line(self.cursor.char_idx);
        let rt = &draw_ctx.render_text;
        let line_height = rt.line_height(self.text_params.scale);
        if self.text_rope.len_chars() > 0 {
            let mut max_lines = (rect.size.y / line_height) as usize;
            max_lines = std::cmp::min(max_lines, self.text_rope.len_lines());
            /*let start_idx = if self.text_rope.len_lines() == 0 {
                0
            } else {
                self.text_rope.line_to_char(self.top_line)
            };
            let end_idx = self.text_rope.line_to_char(self.top_line + max_lines);*/
            rt.draw(
                //self.text_rope.slice(start_idx..end_idx).as_str().unwrap(),
                self.text_rope.bytes(),
                &self.text_params,
                &rect,
                draw_ctx,
            )
        }
        if let Some(_) = self.select_time {
            //let millis = select_time.elapsed().unwrap().as_millis() % 1000;
            //if millis < 500 {
            let before_str = self
                .text_rope
                .slice(self.text_rope.line_to_char(cursor_line)..self.cursor.char_idx)
                .as_str()
                .unwrap();
            let mut cursor_pt1 = Point::new(
                rt.measure(before_str, self.text_params.scale).x / rect.size.x,
                (cursor_line - self.top_line) as f32 * line_height / rect.size.y,
            );
            let mut cursor_pt2 = Point::new(cursor_pt1.x, cursor_pt1.y + line_height / rect.size.y);
            cursor_pt1 = rect
                .transform(&draw_ctx.viewport)
                .model_to_pixel(&cursor_pt1.to_vec4());
            cursor_pt2 = rect
                .transform(&draw_ctx.viewport)
                .model_to_pixel(&cursor_pt2.to_vec4());
            draw_ctx.draw_line(cursor_pt1, cursor_pt2, rgb_to_f32(0, 0, 0), 3.);
            //}
        }
    }
}

impl HandleKey for TextEdit {
    fn handle_key_down(&mut self, kc: &Keycode, ctx: &EventCtx) -> Option<WidgetResponse> {
        let rt = &ctx.draw_ctx.render_text;
        if let Some(ch) = get_char_from_keycode(*kc) {
            self.insert_char(ch, rt);
        } else if let Some(dir) = get_dir_from_keycode(*kc) {
            self.move_cursor(dir);
        } else if *kc == Keycode::Backspace {
            self.delete_char(rt);
        } else {
            return None;
        }
        Some(just_status(WidgetStatus::REDRAW))
    }
}

impl SelectionT for TextEdit {
    fn on_select(&mut self, ctx: &mut EventCtx) -> Option<WidgetResponse> {
        self.select_time = Some(SystemTime::now());
        *ctx.cursor = SystemCursor::IBeam;
        Some(just_status(WidgetStatus::REDRAW))
    }
    fn on_deselect(&mut self, _: &mut EventCtx) -> Option<WidgetResponse> {
        self.select_time = None;
        Some(just_status(WidgetStatus::REDRAW))
    }
    fn handle_key_down(&mut self, kc: &Keycode, ctx: &EventCtx) -> Option<WidgetResponse> {
        let rt = &ctx.draw_ctx.render_text;
        if let Some(ch) = get_char_from_keycode(*kc) {
            self.insert_char(ch, rt);
        } else if let Some(dir) = get_dir_from_keycode(*kc) {
            self.move_cursor(dir);
        } else if *kc == Keycode::Backspace {
            self.delete_char(rt);
        } else {
            return None;
        }
        Some(just_status(WidgetStatus::REDRAW))
    }
    fn log(&self) {
        println!("text edit select")
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

pub struct TextBox {
    //text_edit: TextEdit,
    default_text: String,
    rect: RotateRect,
    num_chars: usize,
}

impl TextBox {
    pub fn new(default_text: &str, num_chars: usize) -> Self {
        TextBox {
            //text_edit: TextEdit::new(default_text, size),
            default_text: String::from(default_text),
            rect: RotateRect::default(),
            num_chars
        }
    }
    /*pub fn new_rotated(default_text: &str, rect: RotateRect) -> Self {
        TextBox {
            default_text: String::from(default_text),
            num_chars: None,
            //text_edit: TextEdit::new(default_text, rect.size),
            rect,
        }
    }*/
}

#[allow(dead_code)]
impl Widget for TextBox {
    fn draw(&self, offset: &Point, ctx: &mut WidgetDrawCtx) {
        let rect = RotateRect {
            offset: *offset,
            ..self.rect.clone()
        };
        rect.builder().color(255, 255, 255).get().draw(ctx.draw_ctx);
        ctx.get_select::<TextEdit>().unwrap().draw(&rect, ctx.draw_ctx);
    }
    fn measure(&self, _: &DrawCtx) -> Point {
        self.rect.size
    }
    fn remeasure(&mut self, ctx: &DrawCtx) -> Point {
         let size = ctx.render_text.measure(
            &String::from_utf8(
                "A".as_bytes()
                    .iter()
                    .cycle()
                    .take(self.num_chars)
                    .map(|c| *c)
                    .collect(),
            )
            .unwrap(),
            1.0,
        );
        self.rect.size = size;
        size
    }
    fn hover(&mut self, _: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        *ctx.cursor = SystemCursor::IBeam;
        Some(just_status(WidgetStatus::FINE))
    }
    fn serialize(&self, buf: &mut MDDoc) {
        /*let text_edit = ctx.select_ctx.get_select::<TextEdit>().unwrap();
        let rope = &text_edit.text_rope;
        let s = rope.slice(0..rope.len_chars()).as_str().unwrap();
        buf.body.extend_from_slice(s.as_bytes())*/
    }
    fn selection(&self) -> Option<Box<dyn SelectionT>> {
        Some(Box::new(TextEdit::new(&self.default_text, self.rect.size)))
    }
    fn click(&mut self, off: &Point, ctx: &mut WidgetEventCtx) -> Option<WidgetResponse> {
        let cursor_pos = ctx.get_select::<TextEdit>().unwrap() 
            .hover_text(off, &self.rect, &ctx.draw_ctx)
            .unwrap_or(0);
        println!("Cursor pos: {:?}", cursor_pos);
        *ctx.cursor = SystemCursor::IBeam;
        let idx = ctx.select_idx().unwrap();
        //Some(just_status(WidgetStatus::REDRAW))
        Some(just_cb(Rc::new(move |app: &mut AppState| {
            let text_edit = app.select_state.get_select_mut::<TextEdit>(idx).
                unwrap();
            text_edit.set_cursor_pos(cursor_pos);
            app.set_select(Some(idx));
        })))
    }
    fn deselect(&mut self) -> Option<WidgetResponse> {
        None
    }
}

pub fn get_char_from_keycode(keycode: Keycode) -> Option<char> {
    let name = keycode.name();
    if name.len() == 1 {
        name.chars().nth(0)
    } else if keycode == Keycode::Space {
        Some(' ')
    } else {
        None
    }
}

pub fn get_dir_from_keycode(kc: Keycode) -> Option<TextCursorDirection> {
    match kc {
        Keycode::Left => Some(TextCursorDirection::Left),
        Keycode::Right => Some(TextCursorDirection::Right),
        Keycode::Up => Some(TextCursorDirection::Up),
        Keycode::Down => Some(TextCursorDirection::Down),
        _ => None,
    }
}
