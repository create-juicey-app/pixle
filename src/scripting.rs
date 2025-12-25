use crate::commands::PaintCommand;
use crate::packages::LoadedTool;
use mlua::prelude::*;
use std::cell::RefCell; // Needed for borrowing UI
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct LuaEngine {
    lua: Lua,
    current_package_path: PathBuf,
}

#[derive(Clone)]
pub enum CursorType {
    SystemCircle,
    CustomImage(String),
}

impl LuaEngine {
    pub fn new() -> Self {
        Self {
            lua: Lua::new(),
            current_package_path: PathBuf::new(),
        }
    }

    pub fn load_tool(&mut self, tool: &LoadedTool) {
        self.current_package_path = tool.package_path.clone();
        let tool_table: LuaTable = self
            .lua
            .load(&tool.script_content)
            .eval()
            .expect(&format!("Failed to load tool: {}", tool.name));
        self.lua.globals().set("current_tool", tool_table).unwrap();
    }

    pub fn get_current_cursor(&self) -> CursorType {
        if let Ok(tool) = self.lua.globals().get::<_, LuaTable>("current_tool") {
            if let Ok(cursor_val) = tool.get::<_, String>("cursor") {
                if cursor_val == "circle" {
                    return CursorType::SystemCircle;
                } else {
                    let full_path = self.current_package_path.join(cursor_val);
                    return CursorType::CustomImage(full_path.to_string_lossy().to_string());
                }
            }
        }
        CursorType::SystemCircle
    }

    // Helper to read "size" from Lua so Rust can draw the cursor ring
    pub fn get_tool_size(&self) -> f32 {
        if let Ok(tool) = self.lua.globals().get::<_, LuaTable>("current_tool") {
            if let Ok(size) = tool.get::<_, f32>("size") {
                return size;
            }
        }
        10.0 // Default fallback
    }

    // --- NEW: The UI Bridge ---
    pub fn draw_ui(&mut self, ui: &mut egui::Ui) {
        self.lua
            .scope(|scope| {
                // Fix: Wrap the UI reference in Rc + RefCell so we can share it
                let ui_handle = Arc::new(RefCell::new(ui));

                let api = self.lua.create_table()?;

                // ui.heading("Text")
                let ui = ui_handle.clone(); // Clone the pointer
                let heading = scope.create_function_mut(move |_, text: String| {
                    ui.borrow_mut().heading(text);
                    Ok(())
                })?;
                api.set("heading", heading)?;

                // ui.label("Text")
                let ui = ui_handle.clone();
                let label = scope.create_function_mut(move |_, text: String| {
                    ui.borrow_mut().label(text);
                    Ok(())
                })?;
                api.set("label", label)?;

                // ui.separator()
                let ui = ui_handle.clone();
                let separator = scope.create_function_mut(move |_, ()| {
                    ui.borrow_mut().separator();
                    Ok(())
                })?;
                api.set("separator", separator)?;

                // val = ui.slider("Label", val, min, max)
                let ui = ui_handle.clone();
                let slider = scope.create_function_mut(
                    move |_, (label, mut val, min, max): (String, f64, f64, f64)| {
                        let mut ui_ref = ui.borrow_mut();
                        ui_ref.horizontal(|ui| {
                            ui.label(label);
                            ui.add(egui::Slider::new(&mut val, min..=max));
                        });
                        Ok(val)
                    },
                )?;
                api.set("slider", slider)?;

                // checked = ui.checkbox("Label", checked)
                let ui = ui_handle.clone();
                let checkbox =
                    scope.create_function_mut(move |_, (label, mut val): (String, bool)| {
                        ui.borrow_mut().checkbox(&mut val, label);
                        Ok(val)
                    })?;
                api.set("checkbox", checkbox)?;

                // clicked = ui.button("Label")
                let ui = ui_handle.clone();
                let button = scope.create_function_mut(move |_, label: String| {
                    Ok(ui.borrow_mut().button(label).clicked())
                })?;
                api.set("button", button)?;

                // Call Tool.on_ui(api)
                if let Ok(tool) = self.lua.globals().get::<_, LuaTable>("current_tool") {
                    if let Ok(on_ui) = tool.get::<_, LuaFunction>("on_ui") {
                        let _: () = on_ui.call(api)?;
                    }
                }
                Ok(())
            })
            .unwrap_or_else(|e| println!("Lua UI Error: {:?}", e));
    }

    // --- UPDATED: No longer takes size/aa arguments ---
    // UPDATE this function signature
    pub fn process_input(
        &self,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
        color: [f32; 3],
    ) -> Vec<PaintCommand> {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let commands_clone = commands.clone();

        let api = self.lua.create_table().unwrap();

        let func = self
            .lua
            .create_function_mut(
                move |_, (x, y, r, g, b, a): (i32, i32, u8, u8, u8, Option<u8>)| {
                    if x >= 0 && y >= 0 {
                        let alpha = a.unwrap_or(255);
                        commands_clone
                            .lock()
                            .unwrap()
                            .push(PaintCommand::DrawPixel {
                                x: x as u32,
                                y: y as u32,
                                r,
                                g,
                                b,
                                a: alpha,
                            });
                    }
                    Ok(())
                },
            )
            .unwrap();
        api.set("draw_pixel", func).unwrap();

        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;

        if let Ok(tool) = self.lua.globals().get::<_, LuaTable>("current_tool") {
            if let Ok(on_paint) = tool.get::<_, LuaFunction>("on_paint") {
                // PASS BOTH COORDINATES TO LUA
                // (api, start_x, start_y, end_x, end_y, r, g, b)
                if let Err(e) =
                    on_paint.call::<_, ()>((api, start_x, start_y, end_x, end_y, r, g, b))
                {
                    println!("Lua Runtime Error: {:?}", e);
                }
            }
        }

        commands.lock().unwrap().clone()
    }
}
