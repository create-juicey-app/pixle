use crate::commands::PaintCommand;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    pub fn new() -> Self {
        let lua = Lua::new();

        // UPDATED SCRIPT: Circular Brush
        let script = r#"
            local Tool = {}

            -- Added 'size' argument
            function Tool.on_paint(api, x, y, size, r, g, b)
                local radius = math.floor(size / 2)
                local radius_sq = radius * radius

                -- Loop bounding box based on radius
                for i = -radius, radius do
                    for j = -radius, radius do
                        -- Pythagorean theorem for a circle
                        if (i * i) + (j * j) <= radius_sq then
                            api.draw_pixel(x + i, y + j, r, g, b)
                        end
                    end
                end
            end

            return Tool
        "#;

        let tool_table: LuaTable = lua
            .load(script)
            .eval()
            .expect("Failed to load default tool");
        lua.globals().set("current_tool", tool_table).unwrap();

        Self { lua }
    }

    // Added 'size' parameter
    pub fn process_input(
        &self,
        tex_x: u32,
        tex_y: u32,
        size: f32,
        color: [f32; 3],
    ) -> Vec<PaintCommand> {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let commands_clone = commands.clone();

        let api = self.lua.create_table().unwrap();

        // API Bridge (Optimization: checking bounds in Rust is faster)
        let func = self
            .lua
            .create_function_mut(move |_, (x, y, r, g, b): (i32, i32, u8, u8, u8)| {
                if x >= 0 && y >= 0 {
                    commands_clone
                        .lock()
                        .unwrap()
                        .push(PaintCommand::DrawPixel {
                            x: x as u32,
                            y: y as u32,
                            r,
                            g,
                            b,
                        });
                }
                Ok(())
            })
            .unwrap();

        api.set("draw_pixel", func).unwrap();

        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;

        let tool: LuaTable = self.lua.globals().get("current_tool").unwrap();
        let on_paint: LuaFunction = tool.get("on_paint").unwrap();

        // Pass size to Lua
        if let Err(e) = on_paint.call::<_, ()>((api, tex_x, tex_y, size, r, g, b)) {
            println!("Lua Runtime Error: {:?}", e);
        }

        let result = commands.lock().unwrap().clone();
        result
    }
}
