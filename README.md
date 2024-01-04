# esp-rs
Generic ESP32 rust code , multi-components

# The challenge of combining versions 
- esp32-hal= version="0.16.0"
- esp-wifi needed or otherwise rom_functions not found
- feature 'async' for esp32-hal implies that IO to GPIO and UART is happening in an async func base, so you cannot define interrupt handlers yourself. 
- mostly I can use NoopRawMutex, when however a channel is used from an interrupt I need CriticalSectionRawMutex