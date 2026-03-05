use eframe::egui;
use rand::prelude::IteratorRandom;


/// A simple struct to represent a vertex in our quiver
#[derive(Clone, Copy)]
struct Vertex {
    pos: egui::Pos2,
}

#[derive(Clone)]
struct Quiver {
    num_vertices: usize,
    weights: [[isize; 200]; 200] // lookup table for the weights
}

impl Quiver {
    fn new_empty() -> Self {
        Quiver {
            num_vertices: 0,
            weights: [[0;200];200],
        }
    }

    fn add_vertex(&mut self) {
        self.num_vertices += 1;
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        self.weights[from][to] += 1;
        self.weights[to][from] -= 1;
    }

    fn mutate_at(&mut self, k: usize) {
        // i -> k -> j: add edges:
        for i in 0..self.num_vertices {
            if i == k { continue };
            for j in 0..self.num_vertices {
                if j == k || j == i { continue };
                let ik = self.weights[i][k];
                let kj = self.weights[k][j];
                if ik > 0 && kj > 0 {
                    self.weights[i][j] += ik*kj;
                    self.weights[j][i] -= ik*kj;
                }
            }
        }
        // flip directions at k
        for i in 0..self.num_vertices-1 {
            self.weights[i][k] = -self.weights[i][k];
            self.weights[k][i] = -self.weights[k][i];
        }
    }
}

/// The main application state
struct QuiverApp {
    vertices: Vec<Vertex>,
    quiver: Quiver,
    
    // State to keep track of the first vertex clicked when trying to draw an edge
    selected_for_edge: Option<usize>,

    // NEW: State to keep track of the vertex currently being dragged.
    // This prevents losing the vertex when moving the mouse too quickly.
    dragged_vertex: Option<usize>,

    // NEW: A stack to remember previous states for the undo functionality.
    // We store a tuple of (Vertices, Edges) representing a snapshot in time.
    history: Vec<(Vec<Vertex>, Quiver)>,
    future: Vec<(Vec<Vertex>, Quiver)>,
}

impl Default for QuiverApp {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            quiver: Quiver::new_empty(),
            selected_for_edge: None,
            dragged_vertex: None, // Initialize with no vertex being dragged
            history: Vec::new(), // Initialize the history stack empty
            future: Vec::new(), // Initialize the future stack empty
        }
    }
}

impl QuiverApp {
    /// NEW: Helper function to save the current state before making changes
    fn save_state(&mut self) {
        self.history.push((self.vertices.clone(), self.quiver.clone()));
    }

    /// Applies a quiver mutation at the given vertex index
    fn mutate_at(&mut self, k: usize) {
        self.quiver.mutate_at(k);
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
            ui.label("- Ctrl + Left-click on empty space to add a new vertex there.");

            ui.horizontal(|ui| {
                if ui.button("Add Vertex").clicked() {
                    self.save_state(); // Save state before adding a vertex
                    let center = ctx.content_rect().center();
                    let offset = (self.vertices.len() as f32 * 15.0) % 60.0; 
                    
                    self.vertices.push(Vertex {
                        pos: egui::pos2(center.x + offset, center.y + offset),
                    });
                    self.quiver.add_vertex();
                }
                
                if ui.button("Clear All").clicked() {
                    self.save_state(); // Save state before clearing
                    self.quiver = Quiver::new_empty();
                    self.vertices.clear();
                    self.selected_for_edge = None;
                }

                if ui.button("Random Mutation").clicked() {
                    for _ in 0..100 {
                        if !self.vertices.is_empty() {
                            self.save_state(); // Save state before random mutation
                            let mut rng = rand::rng();
                            let random_vertex = (0..self.vertices.len()).choose(&mut rng).unwrap();
                            self.mutate_at(random_vertex);
                        }
                    }
                }

                // The button is only enabled if there is something in the history stack
                if ui.add_enabled(!self.history.is_empty(), egui::Button::new("Undo")).clicked() {
                    if let Some((old_vertices, old_quiver)) = self.history.pop() {
                        self.future.push((self.vertices.clone(), self.quiver.clone())); // Save the undone state for potential redo
                        self.vertices = old_vertices;
                        self.quiver = old_quiver;
                        self.selected_for_edge = None; // Reset interaction state to prevent bugs
                    }
                }

                if ui.add_enabled(!self.future.is_empty(), egui::Button::new("Redo")).clicked() {
                    if let Some((old_vertices, old_quiver)) = self.future.pop() {
                        self.history.push((self.vertices.clone(), self.quiver.clone())); // Save the redone state for potential undo
                        self.vertices = old_vertices;
                        self.quiver = old_quiver;
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

            if self.quiver.num_vertices == 0 { return };

            // Draw Edges and their Multiplicities
            for from in 0..self.quiver.num_vertices {
                for to in 0..self.quiver.num_vertices {
                    let weight = self.quiver.weights[from][to];
                    if weight <= 0 { continue };
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

                    let mid_pos = p1 + dir * (p1.distance(p2) * 0.5);
                    let text_pos = mid_pos + normal * 12.0; // Offset perpendicular to the line

                    // Draw a small background for the text so it's readable over other lines
                    painter.rect_filled(
                        egui::Rect::from_center_size(text_pos, egui::vec2(14.0, 14.0)),
                        2.0, // slight rounding
                        egui::Color32::WHITE.linear_multiply(0.8),
                    );

                    let weight_str = if weight>1 { weight.to_string() } else { "".to_string() };

                    painter.text(
                        text_pos,
                        egui::Align2::CENTER_CENTER,
                        weight_str,
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
                let is_ctrl_down = ui.input(|i| i.modifiers.ctrl);
                if let Some(clicked_idx) = hovered_vertex {

                    if is_ctrl_down {
                        self.save_state(); // Save state before mutation
                        self.mutate_at(clicked_idx);
                        self.selected_for_edge = None;
                    } else {
                        if let Some(start_idx) = self.selected_for_edge {
                            if start_idx != clicked_idx {
                                self.save_state(); // Save state before establishing a new edge
                                self.quiver.add_edge(start_idx, clicked_idx);
                            }
                            self.selected_for_edge = None;
                        } else {
                            self.selected_for_edge = Some(clicked_idx);
                        }
                    }
                } else {
                    self.selected_for_edge = None;
                    // check if ctrl is down, if so, add a vertex:
                    if is_ctrl_down {
                        // Extract the exact position where the user clicked
                        if let Some(click_pos) = pointer_pos {
                            self.save_state(); // Save state before adding a vertex
                            self.vertices.push(Vertex {
                                pos: click_pos, 
                            });
                            self.quiver.add_vertex();
                        }
                    }
                }
            }

            // 5. Handle Dragging
            // When the drag starts, we "grab" the vertex currently under the pointer
            if response.drag_started() {
                self.dragged_vertex = hovered_vertex;
            }

            // While dragging, we update the position of the grabbed vertex
            if response.dragged() {
                if let Some(dragged_idx) = self.dragged_vertex {
                    // Note: We intentionally do NOT save state during dragging to avoid 
                    // flooding the undo stack with micro-movements.
                    self.vertices[dragged_idx].pos += ui.input(|i| i.pointer.delta());
                }
            }

            // When the drag is released, we let go of the vertex
            if response.drag_stopped() {
                self.dragged_vertex = None;
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