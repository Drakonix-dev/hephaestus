use raw_window_handle::{DisplayHandle as DHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle as WHandle};

pub(crate) struct WindowHandle<'a> {
    display_handle: DHandle<'a>,
    window_handle: WHandle<'a>,
}

impl<'a> WindowHandle<'a> {
    pub fn new(
        dh: RawDisplayHandle,
        wh: RawWindowHandle,
    ) -> Self {
        let display_handle = unsafe {
            DHandle::borrow_raw(dh)  
        };
        let window_handle = unsafe {
            WHandle::borrow_raw(wh)
        };
        
        Self {
            display_handle,
            window_handle,
        }
    }
}

impl<'a> HasDisplayHandle for WindowHandle<'a> {
    fn display_handle(&self) -> Result<DHandle<'_>, HandleError> {
        Ok(self.display_handle)
    }   
}

impl<'a> HasWindowHandle for WindowHandle<'a> {
    fn window_handle(&self) -> Result<WHandle<'_>, HandleError> {
        Ok(self.window_handle)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowInfo {
    pub(crate) width: u32,
    pub(crate) height: u32,
}
