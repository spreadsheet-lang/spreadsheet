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

const STYLE: &str = include_str!("../assets/style.css");

#[component]
fn Home() -> Element {
    let mut row = use_signal(|| 0_u128);
    let mut col = use_signal(|| 0);

    let col_name = name_from_index(*col.read());

    let left = if *col.read() == 0 {
        rsx! {button { class: "left vcenter", disabled: true, "<" }}
    } else {
        rsx! {button { class: "left vcenter", onclick: move |_| col -= 1, "<" }}
    };
    let up = if *row.read() == 0 {
        rsx! {button { class: "top hcenter", disabled: true, "^" }}
    } else {
        rsx! {button { class: "top hcenter", onclick: move |_| row -= 1, "^" }}
    };
    rsx! {
        style { {STYLE} }
        div { class: "left top", "{col_name}{row}" }
        div { class: "right top" }
        div { class: "left bottom" }
        div { class: "right bottom" }
        div { id: "center" }
        {left}
        button { class: "right vcenter", onclick: move |_| col += 1, ">" }
        {up}
        button { class: "bottom hcenter", onclick: move |_| row += 1, "v" }
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
