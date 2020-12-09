use serde::{Deserialize, Serialize};

use async_i3ipc::{
    event::{Event, WindowChange, WindowData},
    reply::{Node, NodeType},
    I3,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwayWindow {
    pub id: usize,
    pub app_id: String,
    pub name: String,
    pub output: String,
    pub workspace: String,
    pub class: String,
}

impl SwayWindow {
    fn collect_windows(node: &Node) -> Vec<SwayWindow> {
        let mut this = if node.node_type == NodeType::Con && node.name.is_some() {
            let empty = String::from("");
            let name = node.name.as_ref().unwrap_or(&empty);
            let app_id = node.app_id.as_ref().unwrap_or(&empty);
            let class = node
                .window_properties
                .as_ref()
                .map(|wp| wp.class.to_owned())
                .flatten();

            let win = SwayWindow {
                id: node.id,
                app_id: app_id.to_owned(),
                output: empty.to_owned(),
                workspace: empty.to_owned(),
                class: class.unwrap_or(empty.to_owned()),
                name: name.to_owned(),
            };

            vec![win]
        } else {
            vec![]
        };
        let siblings = node.nodes.iter().fold(vec![], |mut vec, child| {
            let children = SwayWindow::collect_windows(child);
            vec.extend(children);
            vec
        });

        let floating_siblings = node.floating_nodes.iter().fold(vec![], |mut vec, child| {
            let children = SwayWindow::collect_windows(child);
            vec.extend(children);
            vec
        });

        this.extend(siblings);
        this.extend(floating_siblings);
        this
    }

    pub async fn fetch() -> Vec<SwayWindow> {
        // establish a connection to i3 over a unix socket
        let mut connection = I3::connect().await.expect("Connection to Sway failed");

        let tree = connection
            .get_tree()
            .await
            .expect("Unable to fetch tree from sway");

        SwayWindow::collect_windows(&tree)
    }
}
