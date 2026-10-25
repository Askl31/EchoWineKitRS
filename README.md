# EchoWineKit (Rust) — binaire unique d'installation et lancement d'EchoPAC

Version Rust « extrêmement optimisée » de la Voie B : **un seul binaire statique,
zéro dépendance crate** (100 % bibliothèque standard), compilation LTO complète,
qui remplace l'ensemble des scripts shell `../EchoWineKit/`.

```
Préfixe Wine dédié ── Wine wine-crossover (LGPL) ── Rosetta 2 ── D3DMetal ── Metal (GPU)
        ▲
        └── echowinekit  (ce binaire : ~1 Mo, sans runtime, sans dépendance)
```

## Choix d'optimisation

| Aspect | Implémentation |
|---|---|
| Dépendances | **0 crate externe** — aucun tokio/serde/clap : démarrage instantané, surface d'attaque nulle, compilation rapide |
| Compilation | `opt-level=3`, **LTO total**, `codegen-units=1`, `strip`, `panic=abort` |
| Copie du média (~2 Go) | **multi-thread** (cœurs disponibles, max 12), blocs de 4 Mio → progression et débit réels y compris sur `setup.exe` (1 Go) |
| Journalisation | sortie en direct + fichier log simultanés via threads lecteurs |
| Appels système | zéro interpréteur shell intermédiaire : `Command` direct (brew, wine, winetricks…) |
| Chemins Windows/UNIX | conversion directe, sans sous-processus `sed` |

## Compilation sur le Mac

```bash
# 1. Rust (si absent) — https://rustup.rs
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Compiler (aucune dépendance à télécharger, ~5 s)
cd EchoWineKitRS
cargo build --release

# 3. Rendre accessible partout
sudo cp target/release/echowinekit /usr/local/bin/
```

## Utilisation

### Interface graphique (recommandée)

```bash
echowinekit gui    # s'ouvre dans le navigateur (127.0.0.1), 4 étapes guidées
```

Page unique : indicateurs d'état, cartes d'étapes, détection automatique des
médias d'installation (Downloads, Bureau, volumes montés), console de journaux
en direct, boutons Finder. Guide détaillé : `Guide-Installation-Pas-a-Pas.md` (ce dossier).

### Ligne de commande

```bash
echowinekit install-kit                       # Rosetta 2 + Homebrew + wine-crossover + winetricks
echowinekit create-prefix                     # préfixe dédié : win10, VC++2008, d3dcompiler, renderer d3dm
echowinekit install-echopac "~/EchoPac 211.32.1.3"   # toute version d'EchoPAC (détection auto de setup.exe)
echowinekit launch                            # lancement EchoPAC
echowinekit doctor                            # diagnostic complet
echowinekit app-bundle                        # icône EchoPAC dans /Applications
echowinekit uninstall-echopac                 # désinstalle EchoPAC (préfixe conservé)
echowinekit reset-kit                         # supprime préfixe + réglages (journaux conservés)
```

Option globale : `--kit-root <chemin>` (défaut `~/EchoPAC-WineKit`).
Option d'installation : `--debug` (trace Wine maximale dans les journaux).

## Commandes détaillées

- **install-kit** — vérifie Apple Silicon + Rosetta 2 (installe si besoin),
  Homebrew, puis `brew install --cask --no-quarantine wine-crossover`
  (Wine + D3DMetal pré-intégrés) et `winetricks`.
- **create-prefix** — préfixe isolé `~/EchoPAC-WineKit/prefix` : Windows 10 64 bits,
  `corefonts`, `vcrun2008`, `d3dcompiler_47`, puis bascule le renderer Wine sur
  `d3dm` (DirectX 11 → Metal). Chaque étape est tolérante aux échecs et journalisée
  dans `~/EchoPAC-WineKit/logs/`.
- **install-echopac** — copie parallèle du média vers `C:\media`, installe
  `vs90_piaredist` (VC++ 2008), le moteur **SQL Anywhere 17** (MSI), puis lance
  `setup.exe` en interactif. Détecte l'exécutable EchoPAC installé et
  l'enregistre dans `~/EchoPAC-WineKit/echopac.conf`.
- **launch** — arrête tout `wineserver` résiduel puis démarre EchoPAC avec
  l'environnement optimisé (`WINEDEBUG=-all`, mono/gecko neutralisés).
- **doctor** — sonde chaque étage (Rosetta, brew, Wine, préfixe, DLLs, renderer,
  exécutable, journaux) et affiche `[OK]/[!!]/[??]`.
- **app-bundle** — génère `/Applications/EchoPAC.app` appelant ce binaire.

## Statut / limites

- Code **validé par `cargo check` (zéro avertissement)** et le serveur de
  l'interface graphique **testé en conditions réelles** (API complète :
  page, état, lancement de tâches, capture des journaux). La partie
  Wine/installeur reste à valider sur un vrai Mac : prévoir une itération
  avec `doctor` et le guide pas à pas.
- Mêmes limites que la Voie B : sans dongle (aucune protection n'est contournée ;
  si EchoPAC exige la clé USB, voir le dossier `../../Mac-Silicon/`), SQL Anywhere
  sous Wine = point incertain, Rosetta en fin de vie côté Apple.
- Usage recherche / enseignement / relecture — GE ne supporte pas ces environnements.

## Licence

MIT — les briques sous-jacentes restent sous leurs licences respectives
(Wine LGPL, D3DMetal Apple, winetricks LGPL).
