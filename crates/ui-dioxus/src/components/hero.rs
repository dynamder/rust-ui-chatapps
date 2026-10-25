use dioxus::prelude::*;

#[cfg(not(feature = "native"))]
const HEADER_SVG: Asset = asset!("/assets/header.svg");

#[component]
pub fn Hero() -> Element {
    // Blitz (`dioxus-native`) does not rasterize SVG images, so the native build
    // uses a text header while the web/desktop build keeps the Dioxus wordmark.
    #[cfg(feature = "native")]
    let header = rsx! {
        div { id: "hero-title",
            h1 { "Rosé Pine Moon" }
            p { "dioxus-native · Blitz renderer" }
        }
    };
    #[cfg(not(feature = "native"))]
    let header = rsx! {
        img { src: HEADER_SVG, id: "header" }
    };

    rsx! {
        // We can create elements inside the rsx macro with the element name followed by a block of attributes and children.
        div {
            // Attributes should be defined in the element before any children
            id: "hero",
            {header}
            div { id: "links",
                // The RSX macro also supports text nodes surrounded by quotes
                a { href: "https://dioxuslabs.com/learn/0.7/", "📚 Learn Dioxus" }
                a { href: "https://dioxuslabs.com/awesome", "🚀 Awesome Dioxus" }
                a { href: "https://github.com/dioxus-community/", "📡 Community Libraries" }
                a { href: "https://github.com/DioxusLabs/sdk", "⚙️ Dioxus Development Kit" }
                a { href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus", "💫 VSCode Extension" }
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}
