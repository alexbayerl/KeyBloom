

# Real-Time RGB Synchronization for Your Keyboard 🌈

**KeyBloom** offers real-time RGB keyboard synchronization with on-screen colors, powered by OpenRGB and Rust.
<br>Whether you're gaming, watching movies, or simply vibing, KeyBloom makes your keyboard become an immersive part of the experience.

This script is adapted to my use with the G213 keyboard, but offers customizability as needed. 

---

### 🚀 Features
- **Real-Time Screen Sync**: Captures screen colors and translates them into stunning LED effects.
- **Fully Customizable**:
  - Adjust brightness and saturation levels.
  - Define the number of LEDs and transition speeds.
  - Tune thresholds for smooth color transitions.
- **Supports OpenRGB**: Works seamlessly with OpenRGB for device control.
- **User-Friendly Configuration**:
  - Built-in terminal-based UI for quick setup.

---

### 🔧 How It Works
1. Captures the average color of your screen in customizable vertical segments.
2. Uses HSV interpolation for smooth, visually pleasing color transitions.
3. Updates your RGB device via OpenRGB.

---

### 🖥️ Requirements
- **OpenRGB**: Ensure OpenRGB ist installed and the SDK server is running.
- **Rust**: A Rust development environment for building the project.

---

### 📦 Installation
0. [Download, install and start the SDK OpenRGB Server](https://openrgb.org/releases.html)
1. Clone the repository:
   ```bash
   git clone https://github.com/alexbayerl/KeyBloom.git
   ```
2. Build the project:
   ```bash
   cd KeyBloom
   cargo build --release
   ```
3. Run KeyBloom:
   ```bash
   ./target/release/keybloom
   ```

---

### 🤝 Contributions
Contributions are welcome!