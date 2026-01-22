use std::path::Path;

use objc2::{declare_class, mutability, ClassType, DeclaredClass, msg_send, msg_send_id, rc::{Allocated, Retained, autoreleasepool}};
use objc2_foundation::{MainThreadMarker, NSObject, NSString};
use objc2_ui_kit::UIApplication;

use super::app_state::{self, send_occluded_event_for_all_windows, EventWrapper};
use crate::event::Event;

#[derive(Clone)]
pub struct AppDelegateIvars {
    last_url: Option<Retained<NSObject>>,
}

declare_class!(
    pub struct AppDelegate;

    unsafe impl ClassType for AppDelegate {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
        const NAME: &'static str = "WinitApplicationDelegate";
    }

    impl DeclaredClass for AppDelegate {
        type Ivars = AppDelegateIvars;
    }

    // UIApplicationDelegate protocol
    unsafe impl AppDelegate {
        #[method_id(init)]
        fn init(this: Allocated<Self>) -> Option<Retained<Self>> {
            let this = this.set_ivars(AppDelegateIvars {
                last_url: None,
            });
            unsafe { msg_send_id![super(this), init] }
        }

        #[method(application:didFinishLaunchingWithOptions:)]
        fn did_finish_launching(&mut self, _application: &UIApplication, _: *mut NSObject) -> bool {
            app_state::did_finish_launching(MainThreadMarker::new().unwrap());
            self.ivars_mut().last_url = None;
            true
        }

        #[method(applicationDidBecomeActive:)]
        fn did_become_active(&self, _application: &UIApplication) {
            let mtm = MainThreadMarker::new().unwrap();
            app_state::handle_nonuser_event(mtm, EventWrapper::StaticEvent(Event::Resumed))
        }

        #[method(applicationWillResignActive:)]
        fn will_resign_active(&self, _application: &UIApplication) {
            let mtm = MainThreadMarker::new().unwrap();
            app_state::handle_nonuser_event(mtm, EventWrapper::StaticEvent(Event::Suspended))
        }

        #[method(applicationWillEnterForeground:)]
        fn will_enter_foreground(&self, application: &UIApplication) {
            send_occluded_event_for_all_windows(application, false);
        }

        #[method(applicationDidEnterBackground:)]
        fn did_enter_background(&self, application: &UIApplication) {
            send_occluded_event_for_all_windows(application, true);
        }

        #[method(applicationWillTerminate:)]
        fn will_terminate(&self, application: &UIApplication) {
            app_state::terminated(application);
        }

        #[method(applicationDidReceiveMemoryWarning:)]
        fn did_receive_memory_warning(&self, _application: &UIApplication) {
            let mtm = MainThreadMarker::new().unwrap();
            app_state::handle_nonuser_event(mtm, EventWrapper::StaticEvent(Event::MemoryWarning))
        }

        #[method(application:openURL:options:)]
        fn open_url(&mut self, _application: &UIApplication, url: *mut NSObject, _options: *mut NSObject) -> bool {
            unsafe {
                let is_file_url: bool = msg_send![url, isFileURL];
                if is_file_url {
                    autoreleasepool(|pool| {
                        if let Some(last_url) = &self.ivars().last_url {
                            let () = msg_send![last_url, stopAccessingSecurityScopedResource];
                        }
                        self.ivars_mut().last_url = Retained::retain(url);
                        if let Some(url) = &self.ivars().last_url {
                            let _started_access: bool = msg_send![url, startAccessingSecurityScopedResource];
                            let string_obj: Option<Retained<NSString>> = msg_send_id![url, path];
                            if let Some(string_obj) = string_obj {
                                let path = Path::new(string_obj.as_str(pool));
                                if path.exists() {
                                    let mtm = MainThreadMarker::new().unwrap();
                                    app_state::handle_nonuser_event(mtm, EventWrapper::StaticEvent(Event::OpenFile(path.to_path_buf())));
                                    return true;
                                }
                            }
                        }
                        false
                    })
                } else {
                    false
                }
            }
        }
    }
);
