use crate::cdp::CdpClient;
use crate::error::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthConfig {
    pub enabled: bool,
    pub remove_webdriver: bool,
    pub spoof_fingerprint: bool,
    pub canvas_noise: bool,
    pub webgl_spoofing: bool,
    pub plugin_spoofing: bool,
    pub language_spoofing: bool,
    pub human_delays: bool,
    pub min_keystroke_delay_ms: u64,
    pub max_keystroke_delay_ms: u64,
    pub min_click_delay_ms: u64,
    pub max_click_delay_ms: u64,
    pub viewport_randomization: bool,
    pub timezone_spoofing: bool,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            remove_webdriver: true,
            spoof_fingerprint: true,
            canvas_noise: true,
            webgl_spoofing: true,
            plugin_spoofing: true,
            language_spoofing: true,
            human_delays: true,
            min_keystroke_delay_ms: 30,
            max_keystroke_delay_ms: 120,
            min_click_delay_ms: 100,
            max_click_delay_ms: 300,
            viewport_randomization: true,
            timezone_spoofing: true,
        }
    }
}

pub struct StealthManager {
    config: StealthConfig,
    client: CdpClient,
}

impl StealthManager {
    pub fn new(client: CdpClient, config: StealthConfig) -> Self {
        Self { config, client }
    }
    
    pub fn config(&self) -> &StealthConfig {
        &self.config
    }
    
    pub async fn apply_all(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        tracing::info!("Applying stealth configurations");
        
        if let Err(e) = self.apply_stealth_script().await {
            tracing::error!("Failed to apply stealth script: {}", e);
        }
        
        if self.config.viewport_randomization {
            if let Err(e) = self.randomize_viewport().await {
                tracing::error!("Failed to randomize viewport: {}", e);
            }
        }
        
        if self.config.timezone_spoofing {
            if let Err(e) = self.spoof_timezone().await {
                tracing::error!("Failed to spoof timezone: {}", e);
            }
        }
        
        Ok(())
    }
    
    async fn apply_stealth_script(&self) -> Result<()> {
        let script = self.build_stealth_script();
        
        self.client.send_command(
            "Page.addScriptToEvaluateOnNewDocument",
            Some(serde_json::json!({ "source": script }))
        ).await?;
        
        tracing::info!("Stealth script injected");
        Ok(())
    }
    
    fn build_stealth_script(&self) -> String {
        let mut parts = Vec::new();
        
        if self.config.remove_webdriver {
            parts.push(r#"
                Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
                window.navigator.webdriver = undefined;
                delete window.webdriver;
                delete navigator.webdriver;
            "#.to_string());
            
            parts.push(r#"
                delete window.cdc_adoQpoasnfa76pfcZLmcfl_Array;
                delete window.cdc_adoQpoasnfa76pfcZLmcfl_Promise;
                delete window.cdc_adoQpoasnfa76pfcZLmcfl_Symbol;
                delete window.__webdriver_evaluate;
                delete window.__selenium_evaluate;
                delete window.__webdriver_script_function;
                delete window.__webdriver_script_func;
                delete window.__webdriver_script_fn;
                delete window.__fxdriver_evaluate;
                delete window.__driver_unwrapped;
                delete window.__webdriver_unwrapped;
                delete window.__Selenium_IDE_Recorder;
                delete window.__selenium;
                delete window.__driver;
                delete window._selenium;
                delete window.ATOM;
                delete window.BLUETOOTH;
                delete window.BroadcastChannel;
                delete window.Proxy;
                delete window.caches;
                delete window.closure酶_lnm;
                delete window.ngdebug;
                delete window.modifiedCanvasDriver;
                delete window.domAutomationController;
                delete window.domAutomation;
                delete window.__WEBDRIVER_ELEM_CACHE;
                delete window.callPhantom;
                delete window.callPhantom;
                delete window._phantom;
                delete window.phantom;
                delete window.SpookyJS;
                delete window.__phantomas;
                delete window.sinon;
                delete window.\$;
                delete window.jQuery;
                delete window.$$;
                delete window.__getServlet;
                delete window.getComputedStyle;
                delete window.__webflow;
            "#.to_string());
        }
        
        if self.config.spoof_fingerprint {
            let hardware_concurrency = Self::random_choices(&[2, 4, 8, 16], &[0.1, 0.2, 0.4, 0.3]);
            let device_memory = Self::random_choices(&[2, 4, 8, 16], &[0.2, 0.3, 0.3, 0.2]);
            
            parts.push(format!(r#"
                Object.defineProperty(navigator, 'hardwareConcurrency', {{
                    get: () => {},
                    configurable: true
                }});
            "#, hardware_concurrency));
            
            parts.push(format!(r#"
                Object.defineProperty(navigator, 'deviceMemory', {{
                    get: () => {},
                    configurable: true
                }});
            "#, device_memory));
        }
        
        if self.config.canvas_noise {
            parts.push(r#"
                const _originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
                HTMLCanvasElement.prototype.toDataURL = function(...args) {
                    const context = this.getContext('2d');
                    if (context) {
                        const imageData = context.getImageData(0, 0, this.width, this.height);
                        for (let i = 0; i < imageData.data.length; i += 4) {
                            const noise = (Math.random() - 0.5) * 2;
                            imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + noise));
                            imageData.data[i + 1] = Math.max(0, Math.min(255, imageData.data[i + 1] + noise));
                            imageData.data[i + 2] = Math.max(0, Math.min(255, imageData.data[i + 2] + noise));
                        }
                        context.putImageData(imageData, 0, 0);
                    }
                    return _originalToDataURL.apply(this, args);
                };
                
                const _originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;
                CanvasRenderingContext2D.prototype.getImageData = function(...args) {
                    const imageData = _originalGetImageData.apply(this, args);
                    for (let i = 0; i < imageData.data.length; i += 4) {
                        const noise = (Math.random() - 0.5) * 2;
                        imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + noise));
                        imageData.data[i + 1] = Math.max(0, Math.min(255, imageData.data[i + 1] + noise));
                        imageData.data[i + 2] = Math.max(0, Math.min(255, imageData.data[i + 2] + noise));
                    }
                    return imageData;
                };
            "#.to_string());
        }
        
        if self.config.webgl_spoofing {
            parts.push(r#"
                const _getParameter = WebGLRenderingContext.prototype.getParameter;
                WebGLRenderingContext.prototype.getParameter = function(param) {
                    if (param === 37445) return 'Intel Inc.';
                    if (param === 37446) return 'Intel Iris OpenGL Engine';
                    if (param === 7936) {
                        return 'OpenGL ES 2.0 Intel Inc. 2.1 INTEL-10.2.59';
                    }
                    return _getParameter.apply(this, arguments);
                };
                
                const _getExtension = WebGLRenderingContext.prototype.getExtension;
                WebGLRenderingContext.prototype.getExtension = function(name) {
                    if (name === 'WEBGL_debug_renderer_info') {
                        return {
                            UNMASKED_VENDOR_WEBGL: 37445,
                            UNMASKED_RENDERER_WEBGL: 37446,
                            getParameter: (param) => {
                                if (param === 37445) return 'Intel Inc.';
                                if (param === 37446) return 'Intel Iris OpenGL Engine';
                                return null;
                            }
                        };
                    }
                    return _getExtension.call(this, name);
                };
                
                const _getContext = HTMLCanvasElement.prototype.getContext;
                HTMLCanvasElement.prototype.getContext = function(type, attributes) {
                    const context = _getContext.call(this, type, attributes);
                    if (context && type === 'webgl') {
                        const originalGetParameter = context.getParameter.bind(context);
                        context.getParameter = function(param) {
                            if (param === 37445) return 'Intel Inc.';
                            if (param === 37446) return 'Intel Iris OpenGL Engine';
                            return originalGetParameter(param);
                        };
                    }
                    return context;
                };
            "#.to_string());
        }
        
        if self.config.plugin_spoofing {
            parts.push(r#"
                Object.defineProperty(navigator, 'plugins', {
                    get: () => {
                        const plugins = ['Chrome PDF Plugin', 'Chrome PDF Viewer', 'Native Client'];
                        return plugins.map(name => ({
                            name: name,
                            description: name + ' (Portable Document Format)',
                            filename: 'internal-' + name.toLowerCase().replace(/\s+/g, '-') + '-plugin',
                            length: 0
                        }));
                    },
                    configurable: true
                });
                
                Object.defineProperty(navigator, 'mimeTypes', {
                    get: () => {
                        const mimes = [
                            { type: 'application/pdf', suffixes: 'pdf', description: 'Portable Document Format', enabledPlugin: null },
                            { type: 'application/x-google-chrome-pdf', suffixes: 'pdf', description: 'Chrome PDF Plugin', enabledPlugin: null }
                        ];
                        return mimes.map(m => ({
                            type: m.type,
                            suffixes: m.suffixes,
                            description: m.description,
                            enabledPlugin: null
                        }));
                    },
                    configurable: true
                });
            "#.to_string());
        }
        
        if self.config.language_spoofing {
            parts.push(r#"
                Object.defineProperty(navigator, 'languages', {
                    get: () => ['en-US', 'en', 'vi-VN', 'vi', 'fr-FR', 'fr'],
                    configurable: true
                });
                
                const _eval = window.eval;
                window.eval = function(code) {
                    if (code && code.includes('navigator.languages')) {
                        return _eval.call(window, code.replace(
                            /navigator\.languages/g,
                            "['en-US', 'en', 'vi-VN', 'vi', 'fr-FR', 'fr']"
                        ));
                    }
                    return _eval.apply(window, arguments);
                };
            "#.to_string());
        }
        
        parts.push(r#"
            window.chrome = { runtime: {}, loadTimes: function() {}, csi: function() {} };
            Object.defineProperty(window, 'chrome', { value: window.chrome, writable: false, configurable: false });
            
            if (typeof navigator.permissions !== 'undefined') {
                const _query = navigator.permissions.query.bind(navigator.permissions);
                navigator.permissions.query = (parameters) => {
                    if (parameters && parameters.name === 'notifications') {
                        return Promise.resolve({ state: Notification.permission === 'granted' ? 'granted' : 'default' });
                    }
                    return _query(parameters);
                };
            }
            
            const _innerWidth = Object.getOwnPropertyDescriptor(window, 'innerWidth');
            const _outerWidth = Object.getOwnPropertyDescriptor(window, 'outerWidth');
            const _innerHeight = Object.getOwnPropertyDescriptor(window, 'innerHeight');
            const _outerHeight = Object.getOwnPropertyDescriptor(window, 'outerHeight');
        "#.to_string());
        
        format!(
            r#"(function() {{
                'use strict';
                {}
            }})();"#,
            parts.join("\n")
        )
    }
    
    async fn randomize_viewport(&self) -> Result<()> {
        let width = rand::thread_rng().gen_range(1024..1920);
        let height = rand::thread_rng().gen_range(768..1080);
        let scale = rand::thread_rng().gen_range(0.9..1.1);
        
        let params = serde_json::json!({
            "deviceScaleFactor": scale,
            "mobile": false,
            "width": width,
            "height": height
        });
        
        self.client.send_command(
            "Emulation.setDeviceMetricsOverride",
            Some(params)
        ).await?;
        
        tracing::info!("Viewport randomized to {}x{}", width, height);
        Ok(())
    }
    
    async fn spoof_timezone(&self) -> Result<()> {
        let timezones = [
            "America/New_York",
            "America/Los_Angeles",
            "America/Chicago",
            "Europe/London",
            "Europe/Paris",
            "Asia/Tokyo",
            "Asia/Singapore",
        ];
        
        let timezone = timezones[rand::thread_rng().gen_range(0..timezones.len())];
        
        let params = serde_json::json!({
            "timezoneId": timezone
        });
        
        self.client.send_command(
            "Emulation.setTimezoneOverride",
            Some(params)
        ).await?;
        
        tracing::info!("Timezone spoofed to {}", timezone);
        Ok(())
    }
    
    pub async fn human_delay(&self) {
        if !self.config.human_delays {
            return;
        }
        
        let delay = rand::thread_rng().gen_range(
            self.config.min_click_delay_ms..self.config.max_click_delay_ms
        );
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
    
    pub async fn human_keystroke_delay(&self) {
        if !self.config.human_delays {
            return;
        }
        
        let delay = rand::thread_rng().gen_range(
            self.config.min_keystroke_delay_ms..self.config.max_keystroke_delay_ms
        );
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
    
    pub async fn human_typing_delay(&self) {
        if !self.config.human_delays {
            return;
        }
        
        let base_delay = rand::thread_rng().gen_range(
            self.config.min_keystroke_delay_ms..self.config.max_keystroke_delay_ms
        );
        let variation: u64 = rand::thread_rng().gen_range(0..50);
        tokio::time::sleep(Duration::from_millis(base_delay + variation)).await;
    }
    
    fn random_choices<T: Copy>(values: &[T], weights: &[f64]) -> T {
        let total: f64 = weights.iter().sum();
        let mut r: f64 = rand::thread_rng().gen_range(0.0..total);
        
        for (i, weight) in weights.iter().enumerate() {
            r -= weight;
            if r <= 0.0 {
                return values[i];
            }
        }
        
        values[values.len() - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stealth_config_default() {
        let config = StealthConfig::default();
        assert!(config.enabled);
        assert!(config.remove_webdriver);
        assert!(config.human_delays);
        assert_eq!(config.min_keystroke_delay_ms, 30);
        assert_eq!(config.max_keystroke_delay_ms, 120);
    }
    
    #[test]
    fn test_random_choices() {
        let values = [1, 2, 3];
        let weights = [0.5, 0.3, 0.2];
        
        for _ in 0..100 {
            let result = StealthManager::random_choices(&values, &weights);
            assert!(values.contains(&result));
        }
    }
}
