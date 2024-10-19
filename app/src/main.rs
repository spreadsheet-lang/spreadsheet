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
    let left = if *col.read() == 0 {
        rsx! {button { disabled: true, "<" }}
    } else {
        rsx! {button { onclick: move |_| col -= 1, "<" }}
    };
    let up = if *row.read() == 0 {
        rsx! {button { disabled: true, "^" }}
    } else {
        rsx! {button { onclick: move |_| row -= 1, "^" }}
    };
    rsx! {
        div {
            h1 { "{col_name}{row}" }
            {left}
            button { onclick: move |_| col += 1, ">" }
            {up}
            button { onclick: move |_| row += 1, "v" }
        }
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
