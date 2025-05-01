use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Space};
use iced::{Command, Element, Length};
use crate::usb::DecodedUSBData;
use crate::usb::USBDescriptor;
use crate::gui::styles;
use crate::gui::styles::color;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::marker::PhantomData;
use crate::usb::mitm_traffic::{UsbTransaction, UsbTransferType, UsbDirection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficItem {
    pub timestamp: f64,
    pub raw_data: Vec<u8>,
    pub decoded_data: DecodedUSBData,
    #[serde(skip)]
    _phantom: PhantomData<()>, // Add phantom data to help with sizing
}

// Implement necessary methods for TrafficItem
impl TrafficItem {
    pub fn new(timestamp: f64, raw_data: Vec<u8>, decoded_data: DecodedUSBData) -> Self {
        Self {
            timestamp,
            raw_data,
            decoded_data,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TreeNodeId(String);

impl TreeNodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    #[allow(dead_code)]
    id: TreeNodeId,
    children: Vec<TreeNodeId>,
    expanded: bool,
    data: String,
    item_type: TreeNodeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeNodeType {
    Root,
    Device,
    Configuration,
    Interface,
    Endpoint,
    Transaction, // Transaction node (e.g., control transfer)
    Setup,       // Setup packet
    Data,        // Data packet
    Status,      // Status/ACK packet
    BulkTransfer, // Bulk transfer
    InterruptTransfer, // Interrupt transfer
    IsochronousTransfer, // Isochronous transfer
    #[allow(dead_code)]
    ClassRequest, // Class-specific request
    #[allow(dead_code)]
    VendorRequest, // Vendor-specific request
    #[allow(dead_code)]
    StandardRequest, // Standard request
    Unknown,
    #[allow(dead_code)]
    Other,       // Other/Unknown transaction types
}

pub struct TrafficView {
    traffic_data: Vec<TrafficItem>,
    selected_item: Option<usize>,
    filter_text: String,
    auto_scroll: bool,
    tree_nodes: std::collections::HashMap<TreeNodeId, TreeNode>,
    root_nodes: Vec<TreeNodeId>,
    dark_mode: bool,
    capture_active: bool, // Whether traffic capture is currently active
    speed_selection_open: bool, // Whether the speed selection dialog is open
}

#[derive(Debug, Clone)]
pub enum Message {
    #[allow(dead_code)]
    ItemSelected(usize),
    FilterChanged(String),
    ToggleAutoScroll(bool),
    ClearTraffic,
    LoadData(Vec<TrafficItem>),
    ToggleTreeNode(TreeNodeId),
    ToggleDarkMode(bool),
    // New messages for speed change functionality
    OpenSpeedDialog,
    CloseSpeedDialog,
    ChangeSpeed(crate::usb::Speed),
    NoOp,
}

impl TrafficView {
    pub fn new() -> Self {
        Self {
            traffic_data: Vec::new(),
            selected_item: None,
            filter_text: String::new(),
            auto_scroll: true,
            tree_nodes: std::collections::HashMap::new(),
            root_nodes: Vec::new(),
            dark_mode: true, // Default to dark mode for hacker-friendly UI
            capture_active: false, // Default to capture not active
            speed_selection_open: false, // Default to speed selection dialog closed
        }
    }
    
    // Add a method to set the capture active state
    pub fn set_capture_active(&mut self, active: bool) {
        self.capture_active = active;
    }
    
    // Method to check if capture is active
    pub fn is_capture_active(&self) -> bool {
        self.capture_active
    }
    
    // Add a method to clear captured traffic
    pub fn clear_traffic(&mut self) {
        self.traffic_data.clear();
        self.selected_item = None;
        self.tree_nodes.clear();
        self.root_nodes.clear();
    }
    
    // Add a packet to the traffic view
    pub fn add_packet(&mut self, raw_data: Vec<u8>, decoded_data: DecodedUSBData) {
        // Create a timestamp for the packet
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
            
        // Create a new traffic item
        let item = TrafficItem::new(timestamp, raw_data, decoded_data);
        
        // Add to traffic data
        self.traffic_data.push(item);
        
        // If auto-scroll is enabled, select the new item
        if self.auto_scroll {
            self.selected_item = Some(self.traffic_data.len() - 1);
            // Update the tree view for this item
            self.build_tree_for_selected_item();
        }
    }
    
    // Add a USB transaction to the traffic view (for MitM traffic)
    pub fn add_transaction(&mut self, transaction: UsbTransaction) {
        use log::debug;
        
        debug!("Adding transaction ID {} of type {:?}", transaction.id, transaction.transfer_type);
        
        // Create a transaction node
        let transaction_id = format!("tx_{}", transaction.id);
        let node_id = TreeNodeId::new(transaction_id);
        
        // Determine transaction type label and color
        let (type_label, node_type) = match transaction.transfer_type {
            UsbTransferType::Control => {
                debug!("Processing control transfer");
                ("Control Transfer", TreeNodeType::Transaction)
            },
            UsbTransferType::Bulk => {
                debug!("Processing bulk transfer");
                ("Bulk Transfer", TreeNodeType::BulkTransfer)
            },
            UsbTransferType::Interrupt => {
                debug!("Processing interrupt transfer");
                ("Interrupt Transfer", TreeNodeType::InterruptTransfer)
            },
            UsbTransferType::Isochronous => {
                debug!("Processing isochronous transfer");
                ("Isochronous Transfer", TreeNodeType::IsochronousTransfer)
            },
            UsbTransferType::Unknown => {
                debug!("Processing unknown transfer type");
                ("Unknown Transfer", TreeNodeType::Unknown)
            },
        };
        
        // Create node data with direction and endpoint info
        let summary = transaction.get_summary();
        debug!("Transaction summary: {}", &summary);
        
        let data = format!("{}: {} (Endpoint: 0x{:02X})", 
                         type_label,
                         summary,
                         transaction.endpoint);
        
        // Create root node if it doesn't exist
        if self.root_nodes.is_empty() {
            let root_id = TreeNodeId::new("mitm_root");
            self.root_nodes.push(root_id.clone());
            
            let root_node = TreeNode {
                id: root_id.clone(),
                children: Vec::new(),
                expanded: true,
                data: "USB Transactions".to_string(),
                item_type: TreeNodeType::Root,
            };
            
            self.tree_nodes.insert(root_id, root_node);
        }
        
        // Create the transaction node
        let mut transaction_node = TreeNode {
            id: node_id.clone(),
            children: Vec::new(),
            expanded: true,
            data,
            item_type: node_type,
        };
        
        // Add setup packet as child if present
        if let Some(setup) = &transaction.setup_packet {
            let setup_id = TreeNodeId::new(format!("setup_{}", transaction.id));
            let direction_str = match setup.direction {
                UsbDirection::HostToDevice => "Host to Device",
                UsbDirection::DeviceToHost => "Device to Host",
                UsbDirection::Unknown => "Unknown Direction",
            };
            
            let setup_data = format!("Setup Packet: {} (bmRequestType: 0x{:02X}, bRequest: 0x{:02X})",
                                 direction_str, setup.bmRequestType, setup.bRequest);
            
            let setup_node = TreeNode {
                id: setup_id.clone(),
                children: Vec::new(),
                expanded: true,
                data: setup_data,
                item_type: TreeNodeType::Setup,
            };
            
            transaction_node.children.push(setup_id.clone());
            self.tree_nodes.insert(setup_id, setup_node);
        }
        
        // Add data packet as child if present
        if let Some(data_pkt) = &transaction.data_packet {
            let data_id = TreeNodeId::new(format!("data_{}", transaction.id));
            let direction_str = match data_pkt.direction {
                UsbDirection::HostToDevice => "Host to Device",
                UsbDirection::DeviceToHost => "Device to Host",
                UsbDirection::Unknown => "Unknown Direction",
            };
            
            let data_node_data = format!("Data Packet: {} ({} bytes)",
                                     direction_str, data_pkt.data.len());
            
            let data_node = TreeNode {
                id: data_id.clone(),
                children: Vec::new(),
                expanded: true,
                data: data_node_data,
                item_type: TreeNodeType::Data,
            };
            
            transaction_node.children.push(data_id.clone());
            self.tree_nodes.insert(data_id, data_node);
        }
        
        // Add status packet as child if present
        if let Some(status) = &transaction.status_packet {
            let status_id = TreeNodeId::new(format!("status_{}", transaction.id));
            
            let status_data = format!("Status: {} (Endpoint: 0x{:02X})",
                                  status.status, status.endpoint);
            
            let status_node = TreeNode {
                id: status_id.clone(),
                children: Vec::new(),
                expanded: true,
                data: status_data,
                item_type: TreeNodeType::Status,
            };
            
            transaction_node.children.push(status_id.clone());
            self.tree_nodes.insert(status_id, status_node);
        }
        
        // Add the transaction node to the root
        if let Some(root) = self.tree_nodes.get_mut(&self.root_nodes[0]) {
            root.children.push(node_id.clone());
        }
        
        // Add the transaction node to the tree
        self.tree_nodes.insert(node_id, transaction_node);
    }
    
    // Get traffic data for saving
    pub fn get_traffic_data(&self) -> Option<Vec<TrafficItem>> {
        if self.traffic_data.is_empty() {
            None
        } else {
            Some(self.traffic_data.clone())
        }
    }
    
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ItemSelected(index) => {
                self.selected_item = Some(index);
                // When item is selected, build tree nodes for this item
                self.build_tree_for_selected_item();
                Command::none()
            },
            Message::FilterChanged(filter) => {
                self.filter_text = filter;
                Command::none()
            },
            Message::ToggleAutoScroll(auto_scroll) => {
                self.auto_scroll = auto_scroll;
                Command::none()
            },
            Message::ClearTraffic => {
                self.traffic_data.clear();
                self.selected_item = None;
                self.tree_nodes.clear();
                self.root_nodes.clear();
                Command::none()
            },
            Message::LoadData(data) => {
                self.traffic_data = data;
                self.selected_item = if self.traffic_data.is_empty() {
                    None
                } else {
                    Some(0)
                };
                // When data is loaded, build tree for the first item
                self.build_tree_for_selected_item();
                Command::none()
            },
            Message::ToggleTreeNode(node_id) => {
                if let Some(node) = self.tree_nodes.get_mut(&node_id) {
                    node.expanded = !node.expanded;
                }
                Command::none()
            },
            Message::ToggleDarkMode(enabled) => {
                self.dark_mode = enabled;
                Command::none()
            },
            // New messages for speed dialog
            Message::OpenSpeedDialog => {
                self.speed_selection_open = true;
                Command::none()
            },
            Message::CloseSpeedDialog => {
                self.speed_selection_open = false;
                Command::none()
            },
            Message::ChangeSpeed(_) => {
                // This is handled by the app with a custom map_message function
                // We just need to close the dialog here
                self.speed_selection_open = false;
                Command::none()
            },
            Message::NoOp => Command::none(),
        }
    }
    
    fn build_tree_for_selected_item(&mut self) {
        if let Some(index) = self.selected_item {
            if index < self.traffic_data.len() {
                // Clone the descriptors to avoid borrowing issues
                let descriptors = self.traffic_data[index].decoded_data.descriptors.clone();
                self.build_tree_for_descriptors(&descriptors);
            }
        }
    }
    
    fn build_tree_for_descriptors(&mut self, descriptors: &[USBDescriptor]) {
        // Clear previous tree
        self.tree_nodes.clear();
        self.root_nodes.clear();
        
        // Create root node
        let root_id = TreeNodeId::new("root");
        self.root_nodes.push(root_id.clone());
        
        let root_node = TreeNode {
            id: root_id.clone(),
            children: Vec::new(),
            expanded: true,
            data: "USB Descriptors".to_string(),
            item_type: TreeNodeType::Root,
        };
        
        self.tree_nodes.insert(root_id.clone(), root_node);
        
        // Process descriptors and build hierarchy
        let mut current_device_id = None;
        let mut current_config_id = None;
        let mut current_interface_id = None;
        
        for (i, descriptor) in descriptors.iter().enumerate() {
            match descriptor {
                USBDescriptor::Device(dev) => {
                    let node_id = TreeNodeId::new(format!("device_{}", i));
                    let data = format!("Device Descriptor (VID:{:04X} PID:{:04X})", 
                        dev.vendor_id, dev.product_id);
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Device,
                    };
                    
                    // Add as child of root
                    if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id.clone(), node);
                    current_device_id = Some(node_id);
                    // Reset lower-level IDs
                    current_config_id = None;
                    current_interface_id = None;
                },
                USBDescriptor::Configuration(cfg) => {
                    let node_id = TreeNodeId::new(format!("config_{}", i));
                    let data = format!("Configuration Descriptor #{}", cfg.configuration_value);
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Configuration,
                    };
                    
                    // Add as child of device or root
                    if let Some(device_id) = &current_device_id {
                        if let Some(device) = self.tree_nodes.get_mut(device_id) {
                            device.children.push(node_id.clone());
                        }
                    } else if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id.clone(), node);
                    current_config_id = Some(node_id);
                    // Reset lower-level IDs
                    current_interface_id = None;
                },
                USBDescriptor::Interface(iface) => {
                    let node_id = TreeNodeId::new(format!("interface_{}_{}", i, iface.interface_number));
                    let data = format!("Interface #{} (Class: 0x{:02X})", 
                        iface.interface_number, iface.interface_class.get_value());
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Interface,
                    };
                    
                    // Add as child of configuration or root
                    if let Some(config_id) = &current_config_id {
                        if let Some(config) = self.tree_nodes.get_mut(config_id) {
                            config.children.push(node_id.clone());
                        }
                    } else if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id.clone(), node);
                    current_interface_id = Some(node_id);
                },
                USBDescriptor::Endpoint(ep) => {
                    let node_id = TreeNodeId::new(format!("endpoint_{}_{}", i, ep.endpoint_address));
                    let data = format!("Endpoint 0x{:02X} ({})", 
                        ep.endpoint_address,
                        if (ep.endpoint_address & 0x80) != 0 {
                            "IN"
                        } else {
                            "OUT"
                        });
                    
                    let node = TreeNode {
                        id: node_id.clone(),
                        children: Vec::new(),
                        expanded: true,
                        data,
                        item_type: TreeNodeType::Endpoint,
                    };
                    
                    // Add as child of interface, configuration, or root
                    if let Some(interface_id) = &current_interface_id {
                        if let Some(interface) = self.tree_nodes.get_mut(interface_id) {
                            interface.children.push(node_id.clone());
                        }
                    } else if let Some(config_id) = &current_config_id {
                        if let Some(config) = self.tree_nodes.get_mut(config_id) {
                            config.children.push(node_id.clone());
                        }
                    } else if let Some(root) = self.tree_nodes.get_mut(&root_id) {
                        root.children.push(node_id.clone());
                    }
                    
                    self.tree_nodes.insert(node_id, node);
                },
                // Add other descriptor types as needed
                _ => {
                    // Handle other descriptor types here
                }
            }
        }
    }
    
    pub fn clear(&mut self) {
        self.traffic_data.clear();
        self.selected_item = None;
        self.clear_tree_view();
    }
    
    // Clear the tree view representation of traffic
    pub fn clear_tree_view(&mut self) {
        self.tree_nodes.clear();
        self.root_nodes.clear();
        
        // Recreate just the root node for the tree
        let root_id = TreeNodeId::new("root");
        let root_node = TreeNode {
            id: root_id.clone(),
            children: Vec::new(),
            expanded: true,
            data: "USB Traffic".to_string(),
            item_type: TreeNodeType::Root,
        };
        
        self.tree_nodes.insert(root_id, root_node);
    }
    
    // Helper to render a collapsible tree node
    fn render_tree_node(&self, node_id: &TreeNodeId, level: usize) -> Element<Message> {
        if let Some(node) = self.tree_nodes.get(node_id) {
            // Define indentation based on level
            let indent_width = 20.0 * level as f32;
            
            // Create toggle button for expand/collapse with enhanced styling
            let toggle_icon = if node.children.is_empty() {
                "   " // No toggle for leaf nodes
            } else if node.expanded {
                "▼ " // Down triangle for expanded
            } else {
                "▶ " // Right triangle for collapsed
            };
            
            // Determine icon color based on node type for better visual hierarchy
            let icon_color = if self.dark_mode {
                match node.item_type {
                    TreeNodeType::Root => color::dark::PRIMARY_LIGHT,
                    TreeNodeType::Device => color::dark::USB_CYAN, 
                    TreeNodeType::Configuration |
                    TreeNodeType::Interface |
                    TreeNodeType::Endpoint => color::dark::USB_GREEN,
                    TreeNodeType::Transaction |
                    TreeNodeType::BulkTransfer |
                    TreeNodeType::InterruptTransfer |
                    TreeNodeType::IsochronousTransfer => color::dark::USB_YELLOW,
                    _ => color::dark::TEXT_SECONDARY,
                }
            } else {
                match node.item_type {
                    TreeNodeType::Root => color::PRIMARY_LIGHT,
                    TreeNodeType::Device => color::USB_CYAN, 
                    TreeNodeType::Configuration |
                    TreeNodeType::Interface |
                    TreeNodeType::Endpoint => color::USB_GREEN,
                    TreeNodeType::Transaction |
                    TreeNodeType::BulkTransfer |
                    TreeNodeType::InterruptTransfer |
                    TreeNodeType::IsochronousTransfer => color::USB_YELLOW,
                    _ => color::TEXT_SECONDARY,
                }
            };
            
            // Pick appropriate text color based on node type
            let node_color = if self.dark_mode {
                match node.item_type {
                    TreeNodeType::Root => color::dark::TEXT,
                    TreeNodeType::Device => color::dark::PRIMARY,
                    TreeNodeType::Configuration => color::dark::SECONDARY,
                    TreeNodeType::Interface => color::dark::USB_GREEN,
                    TreeNodeType::Endpoint => color::dark::TEXT,
                    TreeNodeType::Transaction => color::dark::PRIMARY,
                    TreeNodeType::Setup => color::dark::SECONDARY,
                    TreeNodeType::Data => color::dark::USB_GREEN,
                    TreeNodeType::Status => color::dark::USB_YELLOW,
                    TreeNodeType::BulkTransfer => color::dark::PRIMARY,
                    TreeNodeType::InterruptTransfer => color::dark::USB_GREEN,
                    TreeNodeType::IsochronousTransfer => color::dark::USB_CYAN,
                    TreeNodeType::ClassRequest => color::dark::USB_YELLOW,
                    TreeNodeType::VendorRequest => color::dark::USB_CYAN,
                    TreeNodeType::StandardRequest => color::dark::PRIMARY,
                    _ => color::dark::TEXT_SECONDARY,
                }
            } else {
                match node.item_type {
                    TreeNodeType::Root => color::TEXT,
                    TreeNodeType::Device => color::PRIMARY,
                    TreeNodeType::Configuration => color::SECONDARY,
                    TreeNodeType::Interface => color::USB_GREEN,
                    TreeNodeType::Endpoint => color::TEXT,
                    TreeNodeType::Transaction => color::PRIMARY,
                    TreeNodeType::Setup => color::SECONDARY,
                    TreeNodeType::Data => color::USB_GREEN,
                    TreeNodeType::Status => color::USB_YELLOW,
                    TreeNodeType::BulkTransfer => color::PRIMARY,
                    TreeNodeType::InterruptTransfer => color::USB_GREEN,
                    TreeNodeType::IsochronousTransfer => color::USB_CYAN,
                    TreeNodeType::ClassRequest => color::USB_YELLOW,
                    TreeNodeType::VendorRequest => color::USB_CYAN,
                    TreeNodeType::StandardRequest => color::PRIMARY,
                    _ => color::TEXT_SECONDARY,
                }
            };
            
            // Use simple ASCII characters for tree connecting lines to avoid font rendering issues
            let connector_symbol = match (level, node.children.is_empty(), node.expanded) {
                (0, _, _) => "", // root level, no connector
                (_, true, _) => "|---", // non-root leaf node
                (_, false, true) => "|---", // non-root expanded node with children
                (_, false, false) => "|---", // non-root collapsed node with children
            };
            
            // Build the row with toggle button and node content with improved visual styling
            let node_row = row![
                // Indentation space with subtle visual guides
                if level > 0 {
                    let guides = container(
                        column![
                            container(Space::with_height(Length::Fill))
                                .width(Length::Fixed(1.0))
                                .height(Length::Fill)
                                .style(if self.dark_mode {
                                    iced::theme::Container::Custom(Box::new(styles::DarkModeTreeGuide))
                                } else {
                                    iced::theme::Container::Custom(Box::new(styles::TreeGuide))
                                })
                        ]
                    )
                    .width(Length::Fixed(indent_width))
                    .height(Length::Fill)
                    .center_x();
                    
                    guides
                } else {
                    container(text(""))
                        .width(Length::Fixed(indent_width))
                        .into()
                },
                
                // Improved connector line with better contrast
                if level > 0 {
                    text(connector_symbol)
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::PRIMARY_LIGHT)
                        } else {
                            iced::theme::Text::Color(color::SECONDARY)
                        })
                        .size(13) // Slightly smaller for better proportions
                } else {
                    text("")
                },
                
                // Enhanced toggle button with better visual feedback
                if !node.children.is_empty() {
                    let btn: Element<Message> = button(
                        container(
                            text(toggle_icon)
                                .style(iced::theme::Text::Color(icon_color))
                                .size(14)
                        )
                        .padding(2)
                        .center_x()
                        .center_y()
                    )
                    .on_press(Message::ToggleTreeNode(node_id.clone()))
                    .style(if self.dark_mode {
                        iced::theme::Button::Custom(Box::new(styles::DarkModeTreeNodeButton))
                    } else {
                        iced::theme::Button::Custom(Box::new(styles::TreeNodeButton))
                    })
                    .width(Length::Fixed(24.0))
                    .height(Length::Fixed(24.0))
                    .into();
                    btn
                } else {
                    container(
                        text(toggle_icon)
                            .style(iced::theme::Text::Color(icon_color))
                            .size(14)
                    )
                    .width(Length::Fixed(24.0))
                    .height(Length::Fixed(24.0))
                    .center_x()
                    .center_y()
                    .into()
                },
                
                // Node content with improved typography and visual weight
                {
                    if node.item_type == TreeNodeType::Root {
                        // Root nodes get bold text with slightly larger size
                        text(&node.data)
                            .style(iced::theme::Text::Color(node_color))
                            .width(Length::Fill)
                            .size(15)
                    } else {
                        // All other nodes use standard styling
                        text(&node.data)
                            .style(iced::theme::Text::Color(node_color))
                            .width(Length::Fill)
                    }
                }
            ]
            .spacing(2) // Tighter spacing for a cleaner look
            .align_items(iced::Alignment::Center) // Better vertical alignment
            .width(Length::Fill);
            
            // Choose the appropriate style based on node type
            let style = match node.item_type {
                TreeNodeType::Root => {
                    if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeHeaderContainer))
                    } else {
                        iced::theme::Container::Box
                    }
                },
                TreeNodeType::Device | 
                TreeNodeType::BulkTransfer | 
                TreeNodeType::InterruptTransfer |
                TreeNodeType::IsochronousTransfer => {
                    if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModePrimaryNodeContainer))
                    } else {
                        iced::theme::Container::Box
                    }
                },
                TreeNodeType::Setup | 
                TreeNodeType::Status |
                TreeNodeType::Data => {
                    if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeSecondaryNodeContainer))
                    } else {
                        iced::theme::Container::Box
                    }
                },
                _ => {
                    if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeTreeNodeContainer))
                    } else {
                        iced::theme::Container::Box
                    }
                }
            };
            
            // Create a container for the node with the chosen style and better spacing
            let vertical_padding = if node.item_type == TreeNodeType::Root {
                5 // More padding for root nodes
            } else if matches!(node.item_type, 
                TreeNodeType::Device | 
                TreeNodeType::Configuration | 
                TreeNodeType::Interface | 
                TreeNodeType::Endpoint) {
                4 // Medium padding for major structural nodes
            } else {
                3 // Standard padding for detail nodes
            };
            
            let node_container = container(node_row)
                .style(style)
                .width(Length::Fill)
                .padding([vertical_padding, 5, vertical_padding, 5]); // [top, right, bottom, left]
            
            // If node is expanded, add children
            if node.expanded && !node.children.is_empty() {
                let mut elements = Vec::new();
                elements.push(node_container.into());
                
                // Add all children recursively
                for child_id in &node.children {
                    elements.push(self.render_tree_node(child_id, level + 1));
                }
                
                Column::with_children(elements).into()
            } else {
                // Just return the node without children
                node_container.into()
            }
        } else {
            // Fallback for missing nodes
            container(text("Node not found")).into()
        }
    }
    
    pub fn view(&self) -> Element<Message> {
        // Title with appropriate color based on dark mode
        let title_color = if self.dark_mode {
            color::dark::PRIMARY
        } else {
            color::PRIMARY
        };
        
        let title = text("USB Traffic")
            .size(24)
            .style(iced::theme::Text::Color(title_color));
            
        let filter_label = text("Filter:")
            .style(if self.dark_mode {
                iced::theme::Text::Color(color::dark::TEXT)
            } else {
                iced::theme::Text::Default
            });
        
        let filter_input = text_input("Enter filter", &self.filter_text)
            .on_input(Message::FilterChanged)
            .padding(5)
            .width(Length::FillPortion(3))
            .style(if self.dark_mode {
                iced::theme::TextInput::Custom(Box::new(styles::DarkModeTextInput))
            } else {
                iced::theme::TextInput::Default
            });
            
        // Dark mode toggle button
        let dark_mode_button = if self.dark_mode {
            button("Light Mode")
                .on_press(Message::ToggleDarkMode(false))
                .style(iced::theme::Button::Custom(Box::new(styles::DarkModePrimaryButton)))
        } else {
            button("Dark Mode")
                .on_press(Message::ToggleDarkMode(true))
                .style(iced::theme::Button::Primary)
        };
            
        let auto_scroll_button = if self.auto_scroll {
            button("Auto-scroll: ON")
                .on_press(Message::ToggleAutoScroll(false))
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(styles::DarkModePrimaryButton))
                } else {
                    iced::theme::Button::Primary
                })
        } else {
            button("Auto-scroll: OFF")
                .on_press(Message::ToggleAutoScroll(true))
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(styles::DarkModeSecondaryButton))
                } else {
                    iced::theme::Button::Secondary
                })
        };
        
        let clear_button = button("Clear")
            .on_press(Message::ClearTraffic)
            .style(iced::theme::Button::Destructive);
            
        // Add Change Speed button when capture is active
        let change_speed_button = if self.capture_active {
            button("Change Device Speed")
                .on_press(Message::OpenSpeedDialog)
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(styles::DarkModePrimaryButton))
                } else {
                    iced::theme::Button::Primary
                })
        } else {
            button("Change Device Speed")
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(styles::DarkModeSecondaryButton))
                } else {
                    iced::theme::Button::Secondary
                })
        };
        
        // Group buttons together
        let action_buttons = row![
            auto_scroll_button,
            change_speed_button,
            clear_button,
        ]
        .spacing(10)
        .align_items(iced::Alignment::Center);
        
        // Group filter controls together
        let filter_controls = container(
            row![
                filter_label,
                filter_input,
            ]
            .spacing(10)
            .align_items(iced::Alignment::Center)
        )
        .style(if self.dark_mode {
            iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
        } else {
            iced::theme::Container::Custom(Box::new(styles::LightModeContainer))
        })
        .padding(5);
        
        // Build the header with better organization and visual hierarchy
        let header = column![
            row![
                title,
                dark_mode_button,
            ]
            .spacing(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill),
            
            row![
                filter_controls.width(Length::FillPortion(3)),
                action_buttons.width(Length::FillPortion(2)),
            ]
            .spacing(15)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill)
        ]
        .spacing(10)
        .width(Length::Fill);
        
        // Define traffic_list as Element to handle different return types
        let traffic_list: Element<Message> = if self.traffic_data.is_empty() {
            // Empty state with capture status
            let capture_message = if self.capture_active {
                "Capturing traffic... Waiting for USB activity."
            } else {
                "No traffic captured yet. Select a device to start capture."
            };
            
            container(
                column![
                    text(capture_message)
                        .width(Length::Fill)
                        .size(16)
                        .horizontal_alignment(iced::alignment::Horizontal::Center)
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::TEXT)
                        } else {
                            iced::theme::Text::Default
                        }),
                    
                    // Add a visual capture status indicator
                    container(
                        text(if self.capture_active { "● Active" } else { "○ Inactive" })
                            .size(14)
                            .style(if self.capture_active {
                                iced::theme::Text::Color(color::dark::SUCCESS)
                            } else {
                                iced::theme::Text::Color(color::dark::TEXT_SECONDARY)
                            })
                    )
                    .padding(10)
                ]
                .spacing(10)
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .style(if self.dark_mode {
                iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
            } else {
                iced::theme::Container::Box
            })
            .into()
        } else {
            // Create the tree view
            let mut tree_content = Column::new().spacing(2);
            
            // Add all root nodes to the tree
            for root_id in &self.root_nodes {
                tree_content = tree_content.push(self.render_tree_node(root_id, 0));
            }
            
            // Wrap in a scrollable container
            scrollable(tree_content)
                .height(Length::Fill)
                .style(if self.dark_mode {
                    iced::theme::Scrollable::Custom(Box::new(styles::DarkModeScrollable))
                } else {
                    iced::theme::Scrollable::Default
                })
                .into()
        };
        
        // Show capture status
        let capture_status = if self.capture_active {
            container(
                row![
                    text("Capturing USB Traffic...")
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::SUCCESS)
                        } else {
                            iced::theme::Text::Color(color::SUCCESS)
                        })
                ]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .padding(5)
            .style(if self.dark_mode {
                iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
            } else {
                iced::theme::Container::Box
            })
        } else {
            container(
                row![
                    text("USB Traffic Capture Idle")
                        .style(if self.dark_mode {
                            iced::theme::Text::Color(color::dark::TEXT_SECONDARY)
                        } else {
                            iced::theme::Text::Color(color::TEXT_SECONDARY)
                        })
                ]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .padding(5)
            .style(if self.dark_mode {
                iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
            } else {
                iced::theme::Container::Box
            })
        };
        
        // Create the speed selection dialog if it's open
        let content = if self.speed_selection_open {
            let speeds = [crate::usb::Speed::High, crate::usb::Speed::Full, crate::usb::Speed::Low, crate::usb::Speed::Super, crate::usb::Speed::SuperPlus];
            
            // Create the dialog content with explanation text
            let dialog_content = column![
                text("Select Attached Device Speed")
                    .size(20)
                    .style(if self.dark_mode {
                        iced::theme::Text::Color(color::dark::PRIMARY_LIGHT)
                    } else {
                        iced::theme::Text::Color(color::PRIMARY_DARK)
                    }),
                
                text("This setting configures Cynthion to match the speed of the attached USB device.\nSelect the speed of the USB device connected to Cynthion's host port.\nChanging the speed requires a reconnection to apply the new setting.")
                    .size(14)
                    .style(if self.dark_mode {
                        iced::theme::Text::Color(color::dark::TEXT)
                    } else {
                        iced::theme::Text::Color(color::TEXT)
                    }),
                
                Space::with_height(Length::Fixed(20.0)),
                
                // Create a button for each speed option
                Column::with_children(
                    speeds.iter().map(|&speed| {
                        button(
                            text(format!("{}", speed))
                                .width(Length::Fill)
                                .horizontal_alignment(iced::alignment::Horizontal::Center)
                        )
                        .width(Length::Fill)
                        .on_press(Message::ChangeSpeed(speed))
                        .style(if self.dark_mode {
                            iced::theme::Button::Custom(Box::new(styles::DarkModePrimaryButton))
                        } else {
                            iced::theme::Button::Primary
                        })
                        .into()
                    }).collect()
                )
                .spacing(10)
                .width(Length::Fill),
                
                Space::with_height(Length::Fixed(20.0)),
                
                button("Cancel")
                    .width(Length::Fill)
                    .on_press(Message::CloseSpeedDialog)
                    .style(if self.dark_mode {
                        iced::theme::Button::Custom(Box::new(styles::DarkModeSecondaryButton))
                    } else {
                        iced::theme::Button::Secondary
                    })
            ]
            .spacing(10)
            .padding(20)
            .width(Length::Fixed(400.0));
            
            // Create a modal dialog with the content
            container(
                container(dialog_content)
                    .style(if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                    } else {
                        iced::theme::Container::Box
                    })
                    .center_x()
                    .center_y()
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(styles::ModalBackground)))
            .into()
        } else {
            // Normal content when not showing dialog
            let main_content = column![
                header,
                capture_status,
                traffic_list
            ]
            .spacing(10)
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill);
            
            main_content.into()
        };
        
        content
    }
}