use ash::vk;

pub struct PhysicalDevice {}

impl PhysicalDevice {
  // Pick the best available Vulkan physical device. This means the highest rated one that is suitable.
  pub fn pick_physical_device(instance: &ash::Instance) -> Option<(vk::PhysicalDevice, vk::PhysicalDeviceProperties, vk::PhysicalDeviceFeatures)> {
    let phys_devs = unsafe { instance.enumerate_physical_devices().expect("Could not enumerate physical devices!") }; // Get all physical devices
    let mut phys_dev: vk::PhysicalDevice = vk::PhysicalDevice::null(); // Create a null physical device
    let mut current_score = 0.0; // Create a score variable
    for p in &phys_devs { // For each physical device
        let score = PhysicalDevice::rate_physical_device(instance, p);
        if score > current_score { // If the score is higher than the current score, set the physical device to this one
            current_score = score;
            phys_dev = *p;
        }
    }
    if phys_dev == vk::PhysicalDevice::null() { // If the physical device is null, return None (this means no suitable devices were found)
        return None;
    } else {
        let props = unsafe { instance.get_physical_device_properties(phys_dev) }; // Get the properties of the physical device
        let feats = unsafe { instance.get_physical_device_features(phys_dev) }; // Get the features of the physical device
        let device_name = String::from(
            unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }
                .to_str()
                .unwrap(),
        ); // Get the name of the physical device

        let driver_major = props.driver_version >> 22; // Get the major version of the driver
        let driver_minor = (props.driver_version >> 12) & 0x3ff; // Get the minor version of the driver
        let driver_patch = props.driver_version & 0xfff; // Get the patch version of the driver

        let api_major = vk::api_version_major(props.api_version);
        let api_minor = vk::api_version_minor(props.api_version);
        let api_patch = vk::api_version_patch(props.api_version);
        let api_variant = vk::api_version_variant(props.api_version);

        println!("[Vulkan-render][info] Using {:?} device {} (driver v{}.{}.{}) with score {}.", props.device_type, device_name, driver_major, driver_minor, driver_patch, current_score);
        println!("[Vulkan-render][info] Device supports Vulkan v{}.{}.{} (variant {}).", api_major, api_minor, api_patch, api_variant);
        return Some((phys_dev, props, feats));
    }
  }

  // Rate device based on its properties (whether its discrete, integrated, etc; how many queues it has, etc)
  // We also check if the device is suitable at all for our needs (Check for hard requirements [things like if it supports geometry shaders, certain extensions, etc])
  pub fn rate_physical_device(instance: &ash::Instance, device: &vk::PhysicalDevice) -> f32 {
    let props = unsafe { instance.get_physical_device_properties(*device) }; // Get the properties of the physical device
    //dbg!(props);
    let features = unsafe { instance.get_physical_device_features(*device) }; // Get the features of the physical device
    //dbg!(features);
    let queuefamilyproperties = unsafe { instance.get_physical_device_queue_family_properties(*device) }; // Get the queue family properties of the physical device
    //dbg!(&queuefamilyproperties);
    
    let mut score = 0.0;

    if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU { // Dedicated local GPU
        score += 1000.0;
    } else if props.device_type == vk::PhysicalDeviceType::VIRTUAL_GPU { // Unknown GPU connected through virtual machine (likely dedicated)
        score += 500.0;
    } else if props.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU { // Integrated local GPU
        score += 250.0;
    }
    // 0 score for CPU and OTHER types

    // Maximum possible size of textures affects graphics quality
    score += props.limits.max_image_dimension2_d as f32;

    // Application can't function without geometry shaders
    // TODO: Actually this is not true. And MoltenVK doesn't support geometry shaders. We might need these for certain features
    // (especially 3D games) so this should be conditionally added if we are actually utilizing those features.
    /*if features.geometry_shader < 1 { // Features are either 0 (not supported) or 1 (supported)
        println!("Device missing geometry shader support, thus your system is not supported!");
        return 0.0;
    }*/

    let mut found_graphics_queue = false; // We need a graphics queue
    let mut found_transfer_queue = false; // We need a transfer queue
    for (_index, qfam) in queuefamilyproperties.iter().enumerate() {
        if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) { // We need a graphics queue with at least one queue
            found_graphics_queue = true;
        }
        if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER) { // We need a transfer queue with at least one queue
            found_transfer_queue = true;
        }
    }

    if !found_graphics_queue || !found_transfer_queue {
        println!("Phys device missing queues");
        return 0.0;
    }

    score
  }
}