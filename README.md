# The Bazaar Helper

A powerful overlay assistant for **The Bazaar**. It leverages image recognition and log analysis to provide real-time monster insights, card upgrade paths, and shop probability data.

![Version](https://img.shields.io/badge/version-0.6.0-gold.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-blue.svg)

---

##  Features

### 1 Smart Monster Recognition
- **Auto-Detection**: Automatically identifies the three monsters on your screen during the encounter phase.
- **Dynamic Scaling**: Analyzes game logs to match monster stats and levels according to your current day (Day 1 to Day 10+).
- **Skill Previews**: Instantly view all monster skills, health, and their unique item drops.

### 2 Real-time Shop Probabilities
- **Accurate Drop Rates**: Displays the current chance of encountering cards of different tiers in the shop:
  - <span style="color: #CD7F32; font-weight: bold;">Bronze</span>
  - <span style="color: #C0C0C0; font-weight: bold;">Silver</span>
  - <span style="color: #FFD700; font-weight: bold;">Gold</span>
  - <span style="color: #B9F2FF; font-weight: bold;">Diamond</span>
- **Interactive Day Grid**: Click through the 2x5 day matrix to preview shop probabilities for future days.

### 3 Upgrade Path Comparison
- **Inline Value Toggling**: Click on any card description to toggle between different quality tiers (Bronze/Silver/Gold/Diamond).
- **Visual Branding**: Card borders and text values use standardized hex colors matching the game's official tiers.

### 4 Data Synchronization
- **Hand & Stash Sync**: Real-time tracking of your current build via hybrid log and image processing.
- **Semantic Highlighting**: Keywords like Poison, Burn, Freeze, and Haste are color-coded for better readability.

---

##  Installation

1. Go to the [Releases](https://github.com/Duangi/BazaarHelper/releases) page.
2. **Windows**: Download \BazaarHelper_x.x.x_x64-setup.exe\ and run the installer.
3. **macOS**: Download \BazaarHelper_x.x.x_x64.dmg\.
   > **Note**: macOS log paths (~/Library/Logs/Tempo Storm/The Bazaar/) are automatically handled.

---

##  How to Use

1. **Window Controls**:
   - Drag the top toolbar to move the window.
   - Resize by dragging the bottom-right corner.
   - Use the Collapse button to save screen space.
2. **Monster Lookup**:
   - The app will auto-sync when you encounter monsters. You can also manually trigger recognition by clicking the ** Recognize Monsters** button.
3. **View Upgrades**:
   - Simply click on a card's description to see how its values scale across different tiers.

---

##  FAQ

- **Recognition Issue?** 
  - Ensure the game window is visible and active. Recommended resolution is 1080p or higher.
- **Are Probabilities Accurate?**
  - Rates are based on game mechanics analysis and community data; intended for strategy reference only.
- **Automatic Updates?**
  - To ensure stability during gameplay, **automatic updates have been removed**. Please manually check for the latest release on GitHub.

---

##  Development

### Dev Mode
\\\powershell
npm run tauri dev
\\\

### Build Production
\\\powershell
# Windows
npm run tauri build -- --target nsis

# macOS (Must run on Mac machine)
npm run tauri build -- --target dmg
\\\

---

##  Contact

If you find a bug or have a suggestion:
- **GitHub Issues**: Open a feedback ticket.
- **QQ**: 309349894

---
*Disclaimer: This is a third-party tool and does not collect any user data.*
