    pub fn start_capture(&mut self) -> Result<()> {
        // Create a channel to receive packets from the device
        let (data_tx, data_rx) = std::sync::mpsc::channel();
        
        // Store the receiver for later use
        self.data_receiver = Some(data_rx);
        
        // If the device is ready, set up the transfer queue immediately
        if self.device_info.vendor_id() == CYNTHION_VID {
            info!("Starting capture on Cynthion device: {:04x}:{:04x} using High Speed mode", 
                  self.device_info.vendor_id(), self.device_info.product_id());
            
            // Try to set the device to capture mode with multiple attempts if needed
            let max_attempts = 3;
            let mut last_error = None;
            
            for attempt in 1..=max_attempts {
                match self.write_request(1, State::new(true, ConnectionSpeed::High).0) {
                    Ok(_) => {
                        info!("Successfully started capture on attempt {}", attempt);
                        
                        // Create a transfer queue for the bulk transfers
                        let queue = TransferQueue::new(
                            &self.interface, 
                            data_tx,
                            ENDPOINT, 
                            NUM_TRANSFERS, 
                            READ_LEN
                        );
                        
                        // Store the transfer queue
                        self.transfer_queue = Some(queue);
                        
                        // Create a background thread to handle transfers
                        self.start_async_processing();
                        
                        return Ok(());
                    },
                    Err(e) => {
                        warn!("Failed to start capture (attempt {}/{}): {}", 
                            attempt, max_attempts, e);
                        last_error = Some(e);
                        
                        // Wait briefly before retrying
                        if attempt < max_attempts {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
            }
            
            // If we've tried multiple times and still failed, report the error
            if let Some(e) = last_error {
                error!("Failed to start capture after {} attempts", max_attempts);
                return Err(anyhow::anyhow!("Failed to start capture: {}", e));
            } else {
                // This shouldn't happen, but just in case
                return Err(anyhow::anyhow!("Failed to start capture after {} attempts", max_attempts));
            }
        } else {
            // Even if no real Cynthion device is connected yet, we'll still indicate capture is prepared
            // This way we'll be ready to capture traffic as soon as a device is connected
            info!("No Cynthion device connected yet, but capture will start when device connects.");
            
            // Store the transmitter for later use when a device connects
            self.pending_data_tx = Some(data_tx);
            
            // Mark that we should start capturing immediately when a device connects
            self.capture_on_connect = true;
            
            return Ok(());
        }
    }
