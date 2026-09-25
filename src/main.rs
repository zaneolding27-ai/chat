use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk, Entry, Instance,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{
    error::Error,
    ffi::{CStr, CString},
    mem::size_of,
    os::raw::c_void,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

type AppResult<T> = Result<T, Box<dyn Error>>;

struct App {
    renderer: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Ash Vulkan Triangle")
            .with_inner_size(LogicalSize::new(800.0, 600.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };

        match unsafe { Renderer::new(window) } {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => {
                eprintln!("failed to initialize Vulkan: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => renderer.framebuffer_resized = true,
            WindowEvent::RedrawRequested => {
                if let Err(error) = unsafe { renderer.draw_frame() } {
                    eprintln!("rendering failed: {error}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(renderer) = self.renderer.as_ref() {
            renderer.window.request_redraw();
        }
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(renderer) = self.renderer.take() {
            unsafe { renderer.destroy() };
        }
    }
}

struct Renderer {
    window: Window,
    instance: Instance,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    debug_utils: Option<debug_utils::Instance>,
    surface_loader: surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    framebuffer_resized: bool,
}

impl Renderer {
    unsafe fn new(window: Window) -> AppResult<Self> {
        let entry = Entry::load()?;
        let app_name = CString::new("Ash Vulkan Triangle")?;
        let engine_name = CString::new("No Engine")?;
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_0);

        let display_handle = window.display_handle()?.as_raw();
        let mut extension_names =
            ash_window::enumerate_required_extensions(display_handle)?.to_vec();
        let validation_available =
            entry
                .enumerate_instance_layer_properties()?
                .iter()
                .any(|layer| {
                    CStr::from_ptr(layer.layer_name.as_ptr())
                        == CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()
                });
        let enable_validation = VALIDATION_ENABLED && validation_available;
        if enable_validation {
            extension_names.push(debug_utils::NAME.as_ptr());
        }

        let validation_layers = [CString::new("VK_LAYER_KHRONOS_validation")?];
        let layer_names: Vec<*const i8> =
            validation_layers.iter().map(|name| name.as_ptr()).collect();
        let mut debug_info = debug_messenger_create_info();
        let mut instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);
        if enable_validation {
            instance_info = instance_info
                .enabled_layer_names(&layer_names)
                .push_next(&mut debug_info);
        }

        let instance = entry.create_instance(&instance_info, None)?;
        let (debug_utils, debug_messenger) = if enable_validation {
            let utils = debug_utils::Instance::new(&entry, &instance);
            let messenger = utils.create_debug_utils_messenger(&debug_info, None)?;
            (Some(utils), Some(messenger))
        } else {
            (None, None)
        };

        let window_handle = window.window_handle()?.as_raw();
        let surface =
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)?;
        let surface_loader = surface::Instance::new(&entry, &instance);
        let (physical_device, queue_family_index) =
            pick_physical_device(&instance, &surface_loader, surface)?;
        let queue_priorities = [1.0_f32];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);
        let device_extensions = [swapchain::NAME.as_ptr()];
        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extensions);
        let device = instance.create_device(physical_device, &device_info, None)?;
        let graphics_queue = device.get_device_queue(queue_family_index, 0);
        let present_queue = graphics_queue;
        let swapchain_loader = swapchain::Device::new(&instance, &device);
        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) = create_swapchain(
            &surface_loader,
            &swapchain_loader,
            physical_device,
            surface,
            queue_family_index,
            &window,
            vk::SwapchainKHR::null(),
        )?;
        let swapchain_image_views =
            create_image_views(&device, &swapchain_images, swapchain_format)?;
        let render_pass = create_render_pass(&device, swapchain_format)?;
        let (pipeline_layout, graphics_pipeline) = create_graphics_pipeline(&device, render_pass)?;
        let framebuffers = create_framebuffers(
            &device,
            render_pass,
            &swapchain_image_views,
            swapchain_extent,
        )?;
        let command_pool = device.create_command_pool(
            &vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
            None,
        )?;
        let command_buffers = device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(framebuffers.len() as u32),
        )?;
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let mut image_available_semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut render_finished_semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut in_flight_fences = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available_semaphores.push(device.create_semaphore(&semaphore_info, None)?);
            render_finished_semaphores.push(device.create_semaphore(&semaphore_info, None)?);
            in_flight_fences.push(device.create_fence(&fence_info, None)?);
        }

        Ok(Self {
            window,
            instance,
            debug_messenger,
            debug_utils,
            surface_loader,
            surface,
            physical_device,
            queue_family_index,
            device,
            graphics_queue,
            present_queue,
            swapchain_loader,
            swapchain,
            swapchain_images,
            swapchain_image_format: swapchain_format,
            swapchain_extent,
            swapchain_image_views,
            render_pass,
            pipeline_layout,
            graphics_pipeline,
            framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            framebuffer_resized: false,
        })
    }

    unsafe fn draw_frame(&mut self) -> AppResult<()> {
        let fence = self.in_flight_fences[self.current_frame];
        self.device.wait_for_fences(&[fence], true, u64::MAX)?;
        let acquire = self.swapchain_loader.acquire_next_image(
            self.swapchain,
            u64::MAX,
            self.image_available_semaphores[self.current_frame],
            vk::Fence::null(),
        );
        let (image_index, suboptimal) = match acquire {
            Ok(value) => value,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };

        self.device.reset_fences(&[fence])?;
        record_command_buffer(
            &self.device,
            self.command_buffers[image_index as usize],
            self.render_pass,
            self.framebuffers[image_index as usize],
            self.swapchain_extent,
            self.graphics_pipeline,
        )?;
        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(std::slice::from_ref(
                &self.command_buffers[image_index as usize],
            ))
            .signal_semaphores(&signal_semaphores);
        self.device
            .queue_submit(self.graphics_queue, &[submit_info], fence)?;

        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        let present = self
            .swapchain_loader
            .queue_present(self.present_queue, &present_info);
        if self.framebuffer_resized
            || suboptimal
            || present == Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
        {
            self.framebuffer_resized = false;
            self.recreate_swapchain()?;
        } else {
            present?;
        }
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }

    unsafe fn recreate_swapchain(&mut self) -> AppResult<()> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        self.device.device_wait_idle()?;
        for framebuffer in self.framebuffers.drain(..) {
            self.device.destroy_framebuffer(framebuffer, None);
        }
        for view in self.swapchain_image_views.drain(..) {
            self.device.destroy_image_view(view, None);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
        let (swapchain, images, format, extent) = create_swapchain(
            &self.surface_loader,
            &self.swapchain_loader,
            self.physical_device,
            self.surface,
            self.queue_family_index,
            &self.window,
            self.swapchain,
        )?;
        self.swapchain = swapchain;
        self.swapchain_images = images;
        self.swapchain_image_format = format;
        self.swapchain_extent = extent;
        self.swapchain_image_views =
            create_image_views(&self.device, &self.swapchain_images, format)?;
        self.framebuffers = create_framebuffers(
            &self.device,
            self.render_pass,
            &self.swapchain_image_views,
            extent,
        )?;
        self.device
            .free_command_buffers(self.command_pool, &self.command_buffers);
        self.command_buffers = self.device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(self.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(self.framebuffers.len() as u32),
        )?;
        Ok(())
    }

    unsafe fn destroy(self) {
        let _ = self.device.device_wait_idle();
        for semaphore in self.image_available_semaphores {
            self.device.destroy_semaphore(semaphore, None);
        }
        for semaphore in self.render_finished_semaphores {
            self.device.destroy_semaphore(semaphore, None);
        }
        for fence in self.in_flight_fences {
            self.device.destroy_fence(fence, None);
        }
        self.device.destroy_command_pool(self.command_pool, None);
        for framebuffer in self.framebuffers {
            self.device.destroy_framebuffer(framebuffer, None);
        }
        self.device.destroy_pipeline(self.graphics_pipeline, None);
        self.device
            .destroy_pipeline_layout(self.pipeline_layout, None);
        self.device.destroy_render_pass(self.render_pass, None);
        for view in self.swapchain_image_views {
            self.device.destroy_image_view(view, None);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
        self.device.destroy_device(None);
        self.surface_loader.destroy_surface(self.surface, None);
        if let (Some(utils), Some(messenger)) = (self.debug_utils, self.debug_messenger) {
            utils.destroy_debug_utils_messenger(messenger, None);
        }
        self.instance.destroy_instance(None);
    }
}

unsafe fn pick_physical_device(
    instance: &Instance,
    surface_loader: &surface::Instance,
    surface: vk::SurfaceKHR,
) -> AppResult<(vk::PhysicalDevice, u32)> {
    for device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(device);
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
            || properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
        {
            for (index, family) in instance
                .get_physical_device_queue_family_properties(device)
                .iter()
                .enumerate()
            {
                let present = surface_loader.get_physical_device_surface_support(
                    device,
                    index as u32,
                    surface,
                )?;
                if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present {
                    return Ok((device, index as u32));
                }
            }
        }
    }
    Err("no suitable Vulkan device found".into())
}

unsafe fn create_swapchain(
    surface_loader: &surface::Instance,
    swapchain_loader: &swapchain::Device,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    queue_family_index: u32,
    window: &Window,
    old_swapchain: vk::SwapchainKHR,
) -> AppResult<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D)> {
    let capabilities =
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?;
    let formats = surface_loader.get_physical_device_surface_formats(physical_device, surface)?;
    let present_modes =
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?;
    let format = formats
        .iter()
        .copied()
        .find(|format| {
            format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(formats[0]);
    let present_mode = present_modes
        .into_iter()
        .find(|mode| *mode == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO);
    let size = window.inner_size();
    let extent = if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: size.width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: size.height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    };
    let image_count =
        (capabilities.min_image_count + 1).min(if capabilities.max_image_count == 0 {
            u32::MAX
        } else {
            capabilities.max_image_count
        });
    let info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old_swapchain);
    let swapchain = swapchain_loader.create_swapchain(&info, None)?;
    let images = swapchain_loader.get_swapchain_images(swapchain)?;
    let _ = queue_family_index;
    Ok((swapchain, images, format.format, extent))
}

unsafe fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> AppResult<Vec<vk::ImageView>> {
    images
        .iter()
        .map(|&image| {
            Ok(device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
                None,
            )?)
        })
        .collect()
}

unsafe fn create_render_pass(
    device: &ash::Device,
    format: vk::Format,
) -> AppResult<vk::RenderPass> {
    let attachment = vk::AttachmentDescription::default()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
    let color_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_ref));
    let dependency = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
    Ok(device.create_render_pass(
        &vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&attachment))
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dependency)),
        None,
    )?)
}

unsafe fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> AppResult<(vk::PipelineLayout, vk::Pipeline)> {
    let vert = include_bytes!(concat!(env!("OUT_DIR"), "/triangle.vert.spv"));
    let frag = include_bytes!(concat!(env!("OUT_DIR"), "/triangle.frag.spv"));
    let vert_module = create_shader_module(device, vert)?;
    let frag_module = create_shader_module(device, frag)?;
    let entry_point = CString::new("main")?;
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry_point),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry_point),
    ];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE);
    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let color_blend = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false);
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .attachments(std::slice::from_ref(&color_blend));
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)?;
    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);
    let pipeline = device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
        .map_err(|(_, error)| error)?[0];
    device.destroy_shader_module(vert_module, None);
    device.destroy_shader_module(frag_module, None);
    Ok((layout, pipeline))
}

unsafe fn create_shader_module(device: &ash::Device, bytes: &[u8]) -> AppResult<vk::ShaderModule> {
    if bytes.len() % size_of::<u32>() != 0 {
        return Err("SPIR-V bytecode length is not divisible by four".into());
    }
    let words: Vec<u32> = bytes
        .chunks_exact(size_of::<u32>())
        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    Ok(device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)?)
}

unsafe fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    views: &[vk::ImageView],
    extent: vk::Extent2D,
) -> AppResult<Vec<vk::Framebuffer>> {
    views
        .iter()
        .map(|&view| {
            Ok(device.create_framebuffer(
                &vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(std::slice::from_ref(&view))
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1),
                None,
            )?)
        })
        .collect()
}

unsafe fn record_command_buffer(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    extent: vk::Extent2D,
    pipeline: vk::Pipeline,
) -> AppResult<()> {
    device.begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())?;
    let clear = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.02, 0.02, 0.04, 1.0],
        },
    };
    device.cmd_begin_render_pass(
        command_buffer,
        &vk::RenderPassBeginInfo::default()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D::default().extent(extent))
            .clear_values(std::slice::from_ref(&clear)),
        vk::SubpassContents::INLINE,
    );
    device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
    let viewport = vk::Viewport::default()
        .width(extent.width as f32)
        .height(extent.height as f32)
        .max_depth(1.0);
    let scissor = vk::Rect2D::default().extent(extent);
    device.cmd_set_viewport(command_buffer, 0, std::slice::from_ref(&viewport));
    device.cmd_set_scissor(command_buffer, 0, std::slice::from_ref(&scissor));
    device.cmd_draw(command_buffer, 3, 1, 0, 0);
    device.cmd_end_render_pass(command_buffer);
    device.end_command_buffer(command_buffer)?;
    Ok(())
}

fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vulkan_debug_callback))
}

unsafe extern "system" fn vulkan_debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _: *mut c_void,
) -> vk::Bool32 {
    let message = CStr::from_ptr((*data).p_message);
    eprintln!("[Vulkan {severity:?}] {}", message.to_string_lossy());
    vk::FALSE
}

fn main() -> AppResult<()> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App { renderer: None })?;
    Ok(())
}
