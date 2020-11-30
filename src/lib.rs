use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, WebGlProgram, WebGlRenderingContext, WebGlShader, WebGlBuffer};

extern crate voronoi;
use voronoi::{voronoi, Point, make_polygons};

use rand::{Rng, prelude::ThreadRng, distributions::Uniform};

trait RandomInit<T, Distribution> {
    fn new_random(rng : ThreadRng, range : Distribution) -> T;
}

impl RandomInit<Point, Uniform<f64>> for Point {
    fn new_random(mut rng : ThreadRng, range : Uniform<f64>) -> Point {
        let x = rng.sample(range);
        let y = rng.sample(range);
        return Point::new(x, y);
    }
}

fn gen_voronoi_lines(points : &Vec<Point>) -> Vec<Vec<Point>> {    
    const BOX_SIZE: f64 =   1.98;
    let vor_diagram = voronoi(points.clone(), BOX_SIZE);
    let vor_polys = make_polygons(&vor_diagram);

    let mut i_poly = 0;
    for poly in &vor_polys {
        console::log_1(&format!("Poly: {}", i_poly).into());
        i_poly += 1;
        for point in poly{
            console::log_1(&format!("({},{})", point.x, point.y).into());
        }
    }

    return vor_polys;
}

fn check_gl_error(context : &WebGlRenderingContext, line : u32) {
    let error = context.get_error();
    let mut die = true;
    let mut name = "";
    match error {
        WebGlRenderingContext::NO_ERROR => die = false,
        WebGlRenderingContext::INVALID_ENUM => name = "INVALID_ENUM",
        WebGlRenderingContext::INVALID_VALUE => name = "INVALID_VALUE",
        WebGlRenderingContext::INVALID_OPERATION => name = "INVALID_OPERATION",
        WebGlRenderingContext::INVALID_FRAMEBUFFER_OPERATION => name = "INVALID_FRAMEBUFFER_OPERATION",
        WebGlRenderingContext::OUT_OF_MEMORY => name = "OUT_OF_MEMORY",
        WebGlRenderingContext::CONTEXT_LOST_WEBGL => name = "CONTEXT_LOST_WEBGL",
        _ => die = false // rest of the uints...
    }

    if die {
        console::log_1( &format!("GL Error! : {} at line {}", name, line).into());
        panic!();
    }
}
macro_rules! gl_error {
    ( $context:expr ) => {
        {
            check_gl_error($context, line!());
        }
    }
}

fn gen_buffer(context : &WebGlRenderingContext, points : &Vec<Point>) -> Option<WebGlBuffer>{
    let buffer = context.create_buffer();
    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, buffer.as_ref());
    gl_error!(context);

    let vert_array = js_sys::Float32Array::new_with_length( (points.len()as u32)*3);

    let mut i = 0;
    console::log_1( &"conversions :".into() );
    for point in points {
        let x : f64 = point.x.into();
        let x32 : f32 = x as f32;
        vert_array.set_index(i, x32);
        i += 1;
        let y : f64 = point.y.into();
        let y32 : f32 = y as f32;
        vert_array.set_index(i, y32);
        i += 1;
        vert_array.set_index(i, 0.0);
        i += 1;
        console::log_1( &format!("{},{}", x, y ).into() );
    }

    console::log_1( &"vert_array:".into() );
    for a in 0..vert_array.length() {
        console::log_1( &format!("{} : {}", a, vert_array.get_index(a) ).into() );
    }

    context.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ARRAY_BUFFER,
        &vert_array,
        WebGlRenderingContext::STATIC_DRAW,
    );

    gl_error!(context);   

    return buffer;
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    
    let context = canvas
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?;


    context.clear_color(0.973, 0.945, 0.906, 1.0);
    context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

    let vert_shader = compile_shader(
        &context,
        WebGlRenderingContext::VERTEX_SHADER,
        r#"
        attribute vec4 position;
        void main() {
            gl_Position = position + vec4(-1,-1,0,0);
            gl_PointSize = 3.0;
        }
    "#,
    )?;
    let frag_shader = compile_shader(
        &context,
        WebGlRenderingContext::FRAGMENT_SHADER,
        r#"
        void main() {
            gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        }
    "#,
    )?;
    let program = link_program(&context, &vert_shader, &frag_shader)?;
    context.use_program(Some(&program));

    let mut rng : ThreadRng = rand::thread_rng();
    let range : Uniform<f64> = Uniform::new(0., 2.);

    let num_points = rng.gen_range(10,100);

    let mut points = vec![];
    points.resize_with(num_points, || -> Point { Point::new_random(rng, range) } );

    let polys = gen_voronoi_lines(&points);
    let mut buffers = Vec::new();
    for poly in polys {
        let buffer = gen_buffer(&context, &poly);
        if buffer.is_some() {
            buffers.push( (buffer.unwrap(), poly.len()*3 ) );
        }
    }

    let vbuffer = gen_buffer(&context, &points);
    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&vbuffer.unwrap()));

    context.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
    context.enable_vertex_attrib_array(0);

    context.draw_arrays(
        WebGlRenderingContext::POINTS,
        0,
        points.len() as i32,
    );
    gl_error!(&context);


    /////////////////////////////////////////////////////

    context.use_program(Some(&program));
    gl_error!(&context);
    
    for buffer in buffers {
        context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer.0));
        gl_error!(&context);
        context.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
        gl_error!(&context);
        context.enable_vertex_attrib_array(0);
        gl_error!(&context);
        context.draw_arrays(
            WebGlRenderingContext::LINE_LOOP,
            0,
            (buffer.1 / 3 ) as i32,
        );
        gl_error!(&context);
        console::log_1(&format!("(buffer size : {})", buffer.1).into());
    }
    

    //
    Ok(())
}

pub fn compile_shader(
    context: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGlRenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}