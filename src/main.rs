use eframe::egui;
use std::collections::HashMap;

/// A simple struct to represent a vertex in our quiver
#[derive(Clone, Copy)]
struct Vertex {
    pos: egui::Pos2,
}

/// A directed edge between two vertices
#[derive(Clone, PartialEq, Eq, Hash)]
struct Edge {
    from: usize, // Index of the source vertex in the vertices list
    to: usize,   // Index of the target vertex in the vertices list
}

/// The main application state
struct QuiverApp {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    
    // State to keep track of the first vertex clicked when trying to draw an edge
    selected_for_edge: Option<usize>,

    // NEW: A stack to remember previous states for the undo functionality.
    // We store a tuple of (Vertices, Edges) representing a snapshot in time.
    history: Vec<(Vec<Vertex>, Vec<Edge>)>,
}

impl Default for QuiverApp {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            selected_for_edge: None,
            history: Vec::new(), // Initialize the history stack empty
        }
    }
}

impl QuiverApp {
    /// NEW: Helper function to save the current state before making changes
    fn save_state(&mut self) {
        self.history.push((self.vertices.clone(), self.edges.clone()));
    }

    /// Applies a quiver mutation at the given vertex index
    fn mutate_at(&mut self, k: usize) {
        // 1. Identify paths: i -> k -> j
        let mut new_edges = Vec::new();

        let incoming_to_k: Vec<_> = self.edges.iter().filter(|e| e.to == k).cloned().collect();
        let outgoing_from_k: Vec<_> = self.edges.iter().filter(|e| e.from == k).cloned().collect();

        // For every pair of (incoming, outgoing), we create a new edge from i to j
        for in_edge in &incoming_to_k {
            for out_edge in &outgoing_from_k {
                if in_edge.from != out_edge.to { // Avoid self-loops
                     new_edges.push(Edge {
                        from: in_edge.from,
                        to: out_edge.to,
                    });
                }
            }
        }

        // Add the newly created edges to our main list
        self.edges.extend(new_edges);

        // 2. Reverse all edges connected to k
        for edge in self.edges.iter_mut() {
            if edge.from == k {
                edge.from = edge.to;
                edge.to = k;
            } else if edge.to == k {
                edge.to = edge.from;
                edge.from = k;
            }
        }

        // 3. Cancel 2-cycles
        self.cancel_two_cycles();
    }

    /// Finds and removes pairs of edges that form a 2-cycle
    fn cancel_two_cycles(&mut self) {
        let mut to_remove = Vec::new();
        let n = self.edges.len();

        for i in 0..n {
            if to_remove.contains(&i) { continue; }
            for j in (i + 1)..n {
                if to_remove.contains(&j) { continue; }
                
                let e1 = &self.edges[i];
                let e2 = &self.edges[j];

                // If e1 goes A -> B and e2 goes B -> A, they form a 2-cycle
                if e1.from == e2.to && e1.to == e2.from {
                    to_remove.push(i);
                    to_remove.push(j);
                    break;
                }
            }
        }

        to_remove.sort_unstable();
        to_remove.dedup();
        for index in to_remove.into_iter().rev() {
            self.edges.remove(index);
        }
    }
}

impl eframe::App for QuiverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Interactive Quiver Mutator");
            ui.label("Instructions:");
            ui.label("- Click 'Add Vertex' to spawn a new point.");
            ui.label("- Drag vertices to move them.");
            ui.label("- Left-click Vertex A, then Vertex B to draw an edge A -> B.");
            ui.label("- Ctrl + Left-click a vertex to perform a mutation at that vertex.");

            ui.horizontal(|ui| {
                if ui.button("Add Vertex").clicked() {
                    self.save_state(); // Save state before adding a vertex
                    let center = ctx.screen_rect().center();
                    let offset = (self.vertices.len() as f32 * 15.0) % 60.0; 
                    
                    self.vertices.push(Vertex {
                        pos: egui::pos2(center.x + offset, center.y + offset),
                    });
                }
                
                if ui.button("Clear All").clicked() {
                    self.save_state(); // Save state before clearing
                    self.vertices.clear();
                    self.edges.clear();
                    self.selected_for_edge = None;
                }

                // NEW: Undo Button
                // The button is only enabled if there is something in the history stack
                if ui.add_enabled(!self.history.is_empty(), egui::Button::new("Undo")).clicked() {
                    if let Some((old_vertices, old_edges)) = self.history.pop() {
                        self.vertices = old_vertices;
                        self.edges = old_edges;
                        self.selected_for_edge = None; // Reset interaction state to prevent bugs
                    }
                }
            });

            // Make the background white
            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);

            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

            let pointer_pos = response.interact_pointer_pos();

            // 1. Group edges by multiplicity so we only draw one line per unique edge
            let mut edge_counts: HashMap<(usize, usize), usize> = HashMap::new();
            for edge in &self.edges {
                *edge_counts.entry((edge.from, edge.to)).or_insert(0) += 1;
            }

            // 2. Draw Edges and their Multiplicities
            for (&(from, to), &count) in &edge_counts {
                let p1 = self.vertices[from].pos;
                let p2 = self.vertices[to].pos;

                // Draw the line
                painter.line_segment(
                    [p1, p2],
                    egui::Stroke::new(2.0, egui::Color32::BLACK),
                );

                let dir = (p2 - p1).normalized();
                let normal = egui::vec2(-dir.y, dir.x);
                
                // Draw the arrowhead
                let arrow_pos = p1 + dir * (p1.distance(p2) * 0.75); 
                let head_size = 8.0;
                painter.line_segment(
                    [arrow_pos, arrow_pos - dir * head_size + normal * (head_size * 0.5)],
                    egui::Stroke::new(2.0, egui::Color32::BLACK),
                );
                painter.line_segment(
                    [arrow_pos, arrow_pos - dir * head_size - normal * (head_size * 0.5)],
                    egui::Stroke::new(2.0, egui::Color32::BLACK),
                );

                // If multiplicity is > 1, draw the number slightly above the middle of the edge
                if count > 1 {
                    let mid_pos = p1 + dir * (p1.distance(p2) * 0.5);
                    let text_pos = mid_pos + normal * 12.0; // Offset perpendicular to the line

                    // Draw a small background for the text so it's readable over other lines
                    painter.rect_filled(
                        egui::Rect::from_center_size(text_pos, egui::vec2(14.0, 14.0)),
                        2.0, // slight rounding
                        egui::Color32::WHITE.linear_multiply(0.8),
                    );

                    painter.text(
                        text_pos,
                        egui::Align2::CENTER_CENTER,
                        count.to_string(),
                        egui::FontId::proportional(14.0),
                        egui::Color32::RED,
                    );
                }
            }

            // Draw a temporary line while connecting vertices
            if let (Some(start_idx), Some(hover_pos)) = (self.selected_for_edge, pointer_pos) {
                let p1 = self.vertices[start_idx].pos;
                painter.line_segment(
                    [p1, hover_pos],
                    egui::Stroke::new(2.0, egui::Color32::RED.linear_multiply(0.5)),
                );
            }

            let vertex_radius = 15.0;
            let mut hovered_vertex = None;

            // 3. Draw and handle interactions for Vertices
            for (i, vertex) in self.vertices.iter_mut().enumerate() {
                let is_hovered = pointer_pos.map_or(false, |pos| pos.distance(vertex.pos) < vertex_radius);
                if is_hovered {
                    hovered_vertex = Some(i);
                }

                let color = if self.selected_for_edge == Some(i) {
                    egui::Color32::from_rgb(255, 165, 0) // Orange for selected
                } else if is_hovered {
                    egui::Color32::LIGHT_BLUE
                } else {
                    egui::Color32::BLUE
                };

                painter.circle_filled(vertex.pos, vertex_radius, color);
                painter.text(
                    vertex.pos,
                    egui::Align2::CENTER_CENTER,
                    i.to_string(),
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );
            }

            // 4. Handle Canvas Interaction
            if response.clicked() {
                if let Some(clicked_idx) = hovered_vertex {
                    let is_ctrl_down = ui.input(|i| i.modifiers.ctrl);

                    if is_ctrl_down {
                        self.save_state(); // Save state before mutation
                        self.mutate_at(clicked_idx);
                        self.selected_for_edge = None;
                    } else {
                        if let Some(start_idx) = self.selected_for_edge {
                            if start_idx != clicked_idx {
                                self.save_state(); // Save state before establishing a new edge
                                self.edges.push(Edge {
                                    from: start_idx,
                                    to: clicked_idx,
                                });
                            }
                            self.selected_for_edge = None;
                        } else {
                            self.selected_for_edge = Some(clicked_idx);
                        }
                    }
                } else {
                    self.selected_for_edge = None;
                }
            }

            // Handle Dragging
            if response.dragged() {
                if let Some(hovered_idx) = hovered_vertex {
                    if let Some(_pos) = pointer_pos {
                        // Note: We intentionally do NOT save state during dragging to avoid 
                        // flooding the undo stack with micro-movements.
                        self.vertices[hovered_idx].pos += ui.input(|i| i.pointer.delta());
                    }
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Quiver Mutator"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Quiver App",
        options,
        Box::new(|_cc| Ok(Box::new(QuiverApp::default()))),
    )
}