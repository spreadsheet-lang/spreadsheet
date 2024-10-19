#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
}

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    let mut row = use_signal(|| 0_u128);
    let mut col = use_signal(|| 0);

    let col_name = name_from_index(*col.read());

    let left_top_style = r#"
        position: absolute;
        top: 0px;
        left: 0px;
        height: 20px;
        width: 20px;
        background-color: red;
        text-align: center;
    "#;
    let right_top_style = r#"
        position: absolute;
        top: 0px;
        right: 0px;
        height: 20px;
        width: 20px;
        background-color: red;
    "#;

    let left_bottom_style = r#"
        position: absolute;
        bottom: 0px;
        left: 0px;
        height: 20px;
        width: 20px;
        background-color: red;
    "#;
    let right_bottom_style = r#"
        position: absolute;
        bottom: 0px;
        right: 0px;
        height: 20px;
        width: 20px;
        background-color: red;
    "#;
    let center_style = r#"
        position: absolute;
        bottom: 20px;
        right: 20px;
        left: 20px;
        right: 20px;
        background-color: yellow;
    "#;
    let top_style = r#"
        position: absolute;
        top: 0px;
        left: 20px;
        right: 20px;
        height: 20px;
    "#;
    let bottom_style = r#"
        position: absolute;
        bottom: 0px;
        left: 20px;
        right: 20px;
        height: 20px;
    "#;
    let left_style = r#"
        position: absolute;
        bottom: 20px;
        left: 0px;
        top: 20px;
        width: 20px;
    "#;
    let right_style = r#"
        position: absolute;
        bottom: 20px;
        right: 0px;
        top: 20px;
        width: 20px;
    "#;
    let left = if *col.read() == 0 {
        rsx! {button { style: "{left_style}", disabled: true, "<" }}
    } else {
        rsx! {button { style: "{left_style}", onclick: move |_| col -= 1, "<" }}
    };
    let up = if *row.read() == 0 {
        rsx! {button { style: "{top_style}", disabled: true, "^" }}
    } else {
        rsx! {button { style: "{top_style}", onclick: move |_| row -= 1, "^" }}
    };
    rsx! {
        div { style: "{left_top_style}", "{col_name}{row}" }
        div { style: "{right_top_style}" }
        div { style: "{left_bottom_style}" }
        div { style: "{right_bottom_style}" }
        div { style: "{center_style}" }
        {left}
        button { style: "{right_style}", onclick: move |_| row += 1, ">" }
        {up}
        button { style: "{bottom_style}", onclick: move |_| col += 1, "v" }
    }
}

fn name_from_index(mut col: u128) -> String {
    let mut s = String::new();
    loop {
        let rem = (col % 26) as u8;
        col /= 26;
        s.insert(0, (b'A' + rem).into());
        if col == 0 {
            break s;
        }
    }
}
