#![allow(non_snake_case)]

use std::num::NonZeroU128;

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
    let mut row = use_signal(|| NonZeroU128::MIN);
    let mut col = use_signal(|| 0);

    let col_name = name_from_index(*col.read());

    let left = if let Some(next_col) = col.read().checked_sub(1) {
        rsx! {button { class: "left vcenter", onclick: move |_| col.set(next_col), "<" }}
    } else {
        rsx! {button { class: "left vcenter", disabled: true, "<" }}
    };
    let up = if let Some(next_row) = row.read().get().checked_sub(1).and_then(NonZeroU128::new) {
        rsx! {button { class: "top hcenter", onclick: move |_| row.set(next_row), "^" }}
    } else {
        rsx! {button { class: "top hcenter", disabled: true, "^" }}
    };
    rsx! {
        style { {STYLE} }
        div { class: "left top", "{col_name}{row}" }
        div { class: "right top" }
        div { class: "left bottom" }
        div { class: "right bottom" }
        div { class: "hcenter vcenter", id: "center", Grid {
            col: *col.read(),
            row: *row.read(),
        } }
        {left}
        button { class: "right vcenter", onclick: move |_| col += 1, ">" }
        {up}
        button { class: "bottom hcenter", onclick: move |_| {
            let next_row = row.read().checked_add(1).unwrap();
            row.set(next_row)
        }, "v" }
    }
}

const DISPLAY_ROWS: u128 = 30;
const DISPLAY_COLS: u128 = 10;

#[component]
fn Grid(col: u128, row: NonZeroU128) -> Element {
    let style: String = (0..DISPLAY_COLS)
        .map(|i| {
            let xpos = i * 100;
            let j = col + i;
            format!(".col{j} {{ left: {xpos}px }}")
        })
        .chain((0..DISPLAY_ROWS).map(|i| {
            let ypos = i * 20;
            let i = row.get() + i;
            format!(".row{i} {{ top: {ypos}px }}")
        }))
        .collect();
    rsx! {
        style {
            {style}
        }
        for i in 0..DISPLAY_ROWS {
            for j in 0..DISPLAY_COLS {
                Cell {
                    row: NonZeroU128::new(row.get() + i).unwrap(),
                    col: col + j,
                }
            }
        }
    }
}
#[component]
fn Cell(col: u128, row: NonZeroU128) -> Element {
    rsx! { input {
        class: "row{row} col{col} cell",
        value: "{name_from_index(col)}{row}",
    }}
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
