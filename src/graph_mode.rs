extern crate sdl2;

use sdl2::keyboard::{Keycode, Mod};
use sdl2::event::Event;
use std::collections::{HashMap, HashSet};
use crate::primitives::*;
use crate::render_text::{TextParams, TextAlign};

type VertexID = usize;

struct Graph {
    adj_matrix: HashMap<VertexID, HashSet<VertexID>>,
    edges: Vec<(VertexID, VertexID)>,
    next_id: VertexID
}

impl Graph {
    fn new() -> Self {
        Graph { adj_matrix: HashMap::new(), edges: Vec::new(), next_id: 0 }
    }
    fn add(&mut self) -> VertexID {
        let next_id = self.next_id;
        self.adj_matrix.insert(next_id, HashSet::new());
        self.next_id += 1;
        next_id
    }
}

struct GraphDrawing {
    positions: HashMap<VertexID, Point>,
    next_position: Point,
    text_params: HashMap<VertexID, TextParams>
}

impl GraphDrawing {
    fn new() -> Self {
        GraphDrawing { 
            positions: HashMap::new(), 
            next_position: Point::origin(),
            text_params: HashMap::new()
        }
    }
    fn place(&mut self, id: VertexID, ctx: &DrawCtx) {
        let radius = ctx.viewport.x / 24.; 
        if self.next_position == Point::origin() {
            self.next_position = Point::new(radius, ctx.viewport.y / 4.);
        }
        self.positions.insert(id, self.next_position);
        if self.next_position.x + 4. * radius >= ctx.viewport.x {
            self.next_position.x = radius; 
            self.next_position.y += 3. * radius; 
        }
        else {
            self.next_position.x += 3. * radius; 
        }
        self.text_params.insert(id, TextParams::new().align(TextAlign::Center));
    }
    pub fn select_circle(&self, pt: &Point, ctx: &DrawCtx) -> Option<VertexID> {
        let radius = ctx.viewport.x / 24.; 
        self.positions.iter().find(|(_, p)| p.dist(&pt) < radius).map(|(id, _)| *id)
    }
}

pub struct GraphModeState {
    graph: Graph,
    drawing: GraphDrawing,
    selection: HashMap<VertexID, bool>,
}

impl GraphModeState {
    pub fn new() -> Self {
        GraphModeState {
            graph: Graph::new(),
            drawing: GraphDrawing::new(),
            selection: HashMap::new(),
        }
    }
    fn add_vert(&mut self, ctx: &DrawCtx) {
        let id = self.graph.add();
        self.selection.insert(id, false);
        self.drawing.place(id, ctx);
    }
    fn add_selected_edges(&mut self) {
        for (v, b) in self.selection.iter() {
            if *b {
                let m = &mut self.graph.adj_matrix.get_mut(&v).unwrap();
                for (vj, bj) in self.selection.iter() {
                    if *bj {
                        if !m.contains(vj)  {
                            m.insert(*vj);
                            self.graph.edges.push((*v, *vj));
                        }
                    }
                }
            }
        }
    }
    pub fn draw(&self, ctx: &DrawCtx) {
        let radius = ctx.viewport.x / 24.; 
        //let blue = rgb_to_f32(0, 0, 255);
        let purple = rgb_to_f32(255, 0, 255);
        let black = rgb_to_f32(0, 0, 0);
        let white = rgb_to_f32(255, 255, 255);
        let text_offset = Point::new(radius, radius / 2.);
        for e in &self.graph.edges {
            ctx.draw_line(self.drawing.positions[&e.0], self.drawing.positions[&e.1], black, 3.);
        }
        for v in self.graph.adj_matrix.keys() {
            let pos = self.drawing.positions[v];
            ctx.draw_circle(radius, pos, white, true);
            let r = Rect{ c1: self.drawing.positions[v] - text_offset, c2: self.drawing.positions[v] + text_offset };
            let rr = RotateRect::from_rect(r, Radians(0.));
            ctx.render_text.draw(&format!("{}", v), &self.drawing.text_params[v], &rr, ctx);
            if self.selection[v] {
                ctx.draw_circle(radius, pos, purple, false);
            }
        }
    }
    fn handle_select(&mut self, pt: &Point, _: &Mod, ctx: &DrawCtx) {
        if let Some(id) = self.drawing.select_circle(&pt, ctx) {
            self.selection.get_mut(&id).map(|b| *b = true);
        }
        else {
            self.selection.values_mut().for_each(|b| *b = false);
        }
    }
    pub fn handle_mouse_event(&mut self, ev: &Event, kmod: &Mod, ctx: &DrawCtx) {
        match *ev {
            Event::MouseButtonDown { x, y, .. } => {
                let pt = Point::new(x as f32, y as f32);
                self.handle_select(&pt, kmod, ctx);
            },
            _ => {}
        }
    }
    pub fn handle_keyboard_event(&mut self, ev: &Event, ctx: &DrawCtx) {
        match *ev {
            Event::KeyDown{ keycode: Some(Keycode::V), .. } => {
                self.add_vert(ctx);
            }
            Event::KeyDown { keycode: Some(Keycode::E), .. } => {
                self.add_selected_edges();
            }
            _ => {}
        }
    }
}