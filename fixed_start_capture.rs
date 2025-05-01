    // Start capturing USB traffic with specified speed
    pub fn start_capture(&mut self) -> Result<()> {
        // Create a channel to receive packets from the device
        let (data_tx, data_rx) = std::sync::mpsc::channel();
        
        // Store the receiver for later use
        self.data_receiver = Some(data_rx);
        
        // If the device is ready, set up the transfer queue immediately
        if self.device_info.vendor_id() == CYNTHION_VID {
            info!("Starting capture on Cynthion device: {:04x}:{:04x} using High Speed mode", 
                  self.device_info.vendor_id(), self.device_info.product_id());
            
            // First ensure the device is not already in capture mode
            // by sending a stop command. This helps reset the device state.
            info!("Resetting Cynthion to ensure clean capture state");
            if let Err(e) = self.write_request(1, State::new(false, ConnectionSpeed::High).0) {
                warn!("Failed to reset Cynthion capture state: {} (continuing anyway)", e);
                // Don't return error, just continue and try to start capture
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                // Wait for the device to reset
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            
            // Initialize transfer queue before starting capture
            // This ensures we're ready to receive data as soon as capture starts
            info!("Initializing USB transfer queue");
            let queue = TransferQueue::new(
                &self.interface, 
                data_tx,
                ENDPOINT, 
                NUM_TRANSFERS, 
                READ_LEN
            );
            
            // Store the transfer queue
            self.transfer_queue = Some(queue);
            
            // Try to set the device to capture mode with multiple attempts if needed
            let max_attempts = 5; // Increase max attempts
            let mut last_error = None;
            let mut success = false;
            
            info!("Commanding Cynthion to start Man-in-the-Middle capture...");
            for attempt in 1..=max_attempts {
                match self.write_request(1, State::new(true, ConnectionSpeed::High).0) {
                    Ok(_) => {
                        info!("Successfully started capture on attempt {}", attempt);
                        success = true;
                        break;
                    },
                    Err(e) => {
                        warn!("Failed to start capture (attempt {}/{})): {}", 
                            attempt, max_attempts, e);
                        last_error = Some(e);
                        
                        // Wait longer between attempts
                        if attempt < max_attempts {
                            let backoff = 100 * attempt; // Increasing backoff
                            std::thread::sleep(std::time::Duration::from_millis(backoff));
                        }
                    }
                }
            }
            
            if success {
                // Create a background thread to handle transfers
                self.start_async_processing();
                
                // Wait a moment to ensure capture is fully initialized
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                info!("Cynthion Man-in-the-Middle mode activated, ready to capture USB traffic");
                return Ok(());
            }
            
            // If we've tried multiple times and still failed, report the error
            if let Some(e) = last_error {
                error!("Failed to start capture after {} attempts", max_attempts);
                // Reset the transfer queue since it failed
                self.transfer_queue = None;
                return Err(anyhow::anyhow!("Failed to start capture: {}", e));
            } else {
                // This shouldn't happen, but just in case
                self.transfer_queue = None;
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