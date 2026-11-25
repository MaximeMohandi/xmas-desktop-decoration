# Rust + DirectComposition plutôt que C#/WPF

sneakyXmas est écrit en Rust avec `windows-rs`, rendu en Direct2D/Direct3D 11 sur une surface DirectComposition, et livré en un `.exe` de quelques Mo sans runtime. Une animation plein écran permanente avec alpha par pixel est coûteuse en CPU avec les fenêtres transparentes de WPF; DirectComposition la confie au GPU et au DWM.

## Considered Options

- **C# + WPF**: connu de l'auteur, mais rendu transparent logiciel coûteux et runtime lourd à embarquer.
- **Electron**: Chromium complet en mémoire en permanence, disproportionné.
- **C++ natif**: techniquement équivalent à Rust avec les mêmes API; écarté pour la sûreté mémoire, un plantage pouvant laisser des sons de Noël orphelins.
