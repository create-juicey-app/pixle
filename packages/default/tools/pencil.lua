local Tool = {}

-- Define Tool State
Tool.cursor = "circle"
Tool.size = 10.0
Tool.antialiasing = true
Tool.opacity = 1.0

-- Define the UI Layout
function Tool.on_ui(ui)
    ui.heading("Pencil Settings")

    -- Slider returns the new value
    Tool.size = ui.slider("Size", Tool.size, 1.0, 50.0)
    Tool.opacity = ui.slider("Opacity", Tool.opacity, 0.0, 1.0)

    -- Checkbox
    Tool.antialiasing = ui.checkbox("Antialiasing", Tool.antialiasing)

    ui.separator()
    if ui.button("Reset Defaults") then
        Tool.size = 10.0
        Tool.opacity = 1.0
        Tool.antialiasing = true
    end
end

-- Helper: Calculate distance squared from point (px,py) to segment (x1,y1)-(x2,y2)
local function dist_sq_to_segment(px, py, x1, y1, x2, y2)
    local l2 = (x2 - x1) ^ 2 + (y2 - y1) ^ 2
    if l2 == 0 then return (px - x1) ^ 2 + (py - y1) ^ 2 end
    local t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / l2
    t = math.max(0, math.min(1, t))
    local proj_x = x1 + t * (x2 - x1)
    local proj_y = y1 + t * (y2 - y1)
    return (px - proj_x) ^ 2 + (py - proj_y) ^ 2
end

function Tool.on_paint(api, x1, y1, x2, y2, r, g, b)
    local radius = Tool.size / 2
    local radius_sq = radius * radius

    -- Determine the Bounding Box of the line segment
    -- We only loop over pixels that COULD be part of the line
    local min_x = math.floor(math.min(x1, x2) - radius) - 1
    local max_x = math.ceil(math.max(x1, x2) + radius) + 1
    local min_y = math.floor(math.min(y1, y2) - radius) - 1
    local max_y = math.ceil(math.max(y1, y2) + radius) + 1

    for i = min_x, max_x do
        for j = min_y, max_y do
            -- Calculate distance to the LINE SEGMENT, not just the point
            local d_sq = dist_sq_to_segment(i, j, x1, y1, x2, y2)

            local alpha = 0

            if Tool.antialiasing then
                local dist = math.sqrt(d_sq)
                if dist <= radius then
                    alpha = 255
                elseif dist <= radius + 1.0 then
                    alpha = (1.0 - (dist - radius)) * 255
                end
            else
                if d_sq <= radius_sq then alpha = 255 end
            end

            alpha = alpha * Tool.opacity

            if alpha > 0 then
                api.draw_pixel(i, j, r, g, b, math.floor(alpha))
            end
        end
    end
end

return Tool
