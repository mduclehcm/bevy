use crate::{
    pass::{
        LoadOp, PassDescriptor, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, StoreOp, TextureAttachment,
    },
    render_graph::{
        nodes::{Camera2dNode, CameraNode, PassNode, WindowSwapChainNode, WindowTextureNode},
        RenderGraph,
    },
    texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage},
    Color,
};
use bevy_app::GetEventReader;
use bevy_window::{WindowCreated, WindowReference, WindowResized};
use legion::prelude::Resources;

pub struct BaseRenderGraphConfig {
    pub add_2d_camera: bool,
    pub add_3d_camera: bool,
    pub add_main_depth_texture: bool,
    pub add_main_pass: bool,
    pub connect_main_pass_to_swapchain: bool,
    pub connect_main_pass_to_main_depth_texture: bool,
}

pub mod node {
    pub const PRIMARY_SWAP_CHAIN: &str = "swapchain";
    pub const CAMERA: &str = "camera";
    pub const CAMERA2D: &str = "camera2d";
    pub const MAIN_DEPTH_TEXTURE: &str = "main_pass_depth_texture";
    pub const MAIN_PASS: &str = "main_pass";
}

impl Default for BaseRenderGraphConfig {
    fn default() -> Self {
        BaseRenderGraphConfig {
            add_2d_camera: true,
            add_3d_camera: true,
            add_main_pass: true,
            add_main_depth_texture: true,
            connect_main_pass_to_swapchain: true,
            connect_main_pass_to_main_depth_texture: true,
        }
    }
}
/// The "base render graph" provides a core set of render graph nodes which can be used to build any graph.
/// By itself this graph doesn't do much, but it allows Render plugins to interop with each other by having a common
/// set of nodes. It can be customized using `BaseRenderGraphConfig`.
pub trait BaseRenderGraphBuilder {
    fn add_base_graph(
        &mut self,
        resources: &Resources,
        config: &BaseRenderGraphConfig,
    ) -> &mut Self;
}

impl BaseRenderGraphBuilder for RenderGraph {
    fn add_base_graph(
        &mut self,
        resources: &Resources,
        config: &BaseRenderGraphConfig,
    ) -> &mut Self {
        if config.add_3d_camera {
            self.add_system_node(node::CAMERA, CameraNode::default());
        }

        if config.add_2d_camera {
            self.add_system_node(node::CAMERA2D, Camera2dNode::default());
        }

        if config.add_main_depth_texture {
            self.add_node(
                node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::new(
                    WindowReference::Primary,
                    TextureDescriptor {
                        size: Extent3d {
                            depth: 1,
                            width: 1,
                            height: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Depth32Float, // PERF: vulkan docs recommend using 24 bit depth for better performance
                        usage: TextureUsage::OUTPUT_ATTACHMENT,
                    },
                    resources.get_event_reader::<WindowCreated>(),
                    resources.get_event_reader::<WindowResized>(),
                ),
            );
        }

        if config.add_main_pass {
            self.add_node(
                node::MAIN_PASS,
                PassNode::new(PassDescriptor {
                    color_attachments: vec![RenderPassColorAttachmentDescriptor {
                        attachment: TextureAttachment::Input("color".to_string()),
                        resolve_target: None,
                        load_op: LoadOp::Clear,
                        store_op: StoreOp::Store,
                        clear_color: Color::rgb(0.1, 0.1, 0.1),
                    }],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                        attachment: TextureAttachment::Input("depth".to_string()),
                        depth_load_op: LoadOp::Clear,
                        depth_store_op: StoreOp::Store,
                        stencil_load_op: LoadOp::Clear,
                        stencil_store_op: StoreOp::Store,
                        clear_depth: 1.0,
                        clear_stencil: 0,
                    }),
                    sample_count: 1,
                }),
            );

            if config.add_3d_camera {
                self.add_node_edge(node::CAMERA, node::MAIN_PASS).unwrap();
            }

            if config.add_2d_camera {
                self.add_node_edge(node::CAMERA2D, node::MAIN_PASS).unwrap();
            }
        }

        self.add_node(
            node::PRIMARY_SWAP_CHAIN,
            WindowSwapChainNode::new(
                WindowReference::Primary,
                resources.get_event_reader::<WindowCreated>(),
                resources.get_event_reader::<WindowResized>(),
            ),
        );

        if config.connect_main_pass_to_swapchain {
            self.add_slot_edge(
                node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                node::MAIN_PASS,
                "color",
            )
            .unwrap();
        }

        if config.connect_main_pass_to_main_depth_texture {
            self.add_slot_edge(
                node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                node::MAIN_PASS,
                "depth",
            )
            .unwrap();
        }

        self
    }
}