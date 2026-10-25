# Guide d'installation pas à pas — EchoPAC sur Mac Apple Silicon (Voie B, avec interface graphique)

> Écrit pour `EchoWineKitRS` (binaire Rust + interface web locale).
> Durée totale : ~30 à 45 minutes (téléchargements inclus).
> Ce guide vaut pour **n'importe quelle version d'EchoPAC** : 211.32.1.3 comme
> toute version ultérieure fournie sur un autre média (la mise à jour se fait
> par la même opération).

## Ce qu'il vous faut

- Un Mac Apple Silicon (M1 à M4) avec macOS 12 ou plus récent, connecté à Internet.
- Le dossier `EchoWineKitRS` (copié depuis ce PC).
- Le média d'installation EchoPAC : le dossier complet (ex. « EchoPac 211.32.1.3 »)
  ou une image ISO/DMG de n'importe quelle version. Le seul prérequis : il doit
  contenir `setup.exe` (à la racine ou dans un sous-dossier).
- ~10 Go d'espace disque libre.

---

## Étape 0 — Copier les fichiers sur le Mac

Copiez dans `~/Downloads` (ou ailleurs) :
1. le dossier `EchoWineKitRS` (clé USB, AirDrop, réseau…) ;
2. le dossier du média EchoPAC (ex. `~/Downloads/EchoPac 211.32.1.3`).
   Si le média est une ISO : double-cliquez dessus, elle se monte dans `/Volumes`
   et sera détectée automatiquement plus loin.

## Étape 1 — Installer Rust et compiler le logiciel (5 min, une seule fois)

Ouvrez **Terminal** (Applications → Utilitaires → Terminal) :

```bash
# 1. Rust (si déjà installé, cette commande est sans effet)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Compilation (aucune dépendance à télécharger, quelques secondes)
cd ~/Downloads/EchoWineKitRS
cargo build --release

# 3. Rendre la commande accessible partout (optionnel mais pratique)
sudo cp target/release/echowinekit /usr/local/bin/
```

Si `xcode-select` vous réclame des « command line tools », acceptez
(le compilateur Apple est requis par Rust).

## Étape 2 — Lancer l'interface graphique

**Toujours depuis le Terminal** (les mots de passe d'installation
s'y afficheront) :

```bash
echowinekit gui        # ou : ~/Downloads/EchoWineKitRS/target/release/echowinekit gui
```

L'interface s'ouvre dans votre navigateur (adresse locale 127.0.0.1).
Laissez la fenêtre du Terminal ouverte pendant toute la procédure.

Trois indicateurs en haut suivent votre progression :
`Composants (Wine)` → `Préfixe Windows` → `EchoPAC installé`.

## Étape 3 — Installer les composants (une seule fois, ~10 min)

Carte **« 1 · Installer les composants »** → bouton **Installer les composants**.
Cette tâche installe successivement Rosetta 2, Homebrew, Wine « wine-crossover »
(avec D3DMetal pour DirectX 11) et winetricks. **Saisissez votre mot de passe
dans le Terminal quand il est demandé.** La console en bas de l'interface
affiche la progression en direct ; l'indicateur `Composants` passe au vert à la fin.

## Étape 4 — Créer le préfixe Windows (une seule fois, ~5 min)

Carte **« 2 · Créer le préfixe Windows »** → **Créer le préfixe**.
Prépare un « mini-Windows » isolé dans `~/EchoPAC-WineKit/prefix` :
Windows 10 64 bits, VC++ 2008, compilateur de shaders, rendu via Metal.
L'indicateur `Préfixe Windows` passe au vert.

## Étape 5 — Installer EchoPAC (toute version, répétable pour les mises à jour)

Carte **« 3 · Installer ou mettre à jour EchoPAC »** :

1. Dans le champ, indiquez le dossier du média — les dossiers détectés
   automatiquement sont proposés en boutons (Downloads, Bureau, volumes montés).
   Exemple : `~/Downloads/EchoPac 211.32.1.3`
2. Cliquez **Installer**. Le média est copié (progression par jalons de 10 %),
   puis les composants annexes détectés (SQL Anywhere, VC++ redist) sont posés,
   puis **l'installeur GE s'ouvre dans sa propre fenêtre** : suivez-le comme
   sur un PC (compte administrateur, nom du PC ≤ 15 caractères).
3. Si l'installeur propose d'installer un driver **Sentinel/HASP** :
   répondez **Non / Ignorer** (inutile dans cette voie).
4. À la fin, l'indicateur `EchoPAC installé` passe au vert.

### Mettre à jour EchoPAC avec un nouveau média

Exactement la même opération : fournissez le **nouveau** dossier d'installation
(n'importe quelle version) à l'étape 5 — l'installation se fait par-dessus
l'existante. En cas de refus de l'installeur GE, désinstallez d'abord depuis le
même outil (il reprendra le média de l'ancienne version), puis relancez
l'étape 5 avec le nouveau média.

## Étape 6 — Lancer EchoPAC

Carte **« 4 · Lancer EchoPAC »** → gros bouton vert. EchoPAC s'ouvre en plein
écran dans sa propre fenêtre ; la tâche reste « en cours » tant que l'application
est ouverte (c'est normal).

Pour un lancement depuis le Dock / Spotlight : bouton **Créer EchoPAC.app**
(une icône EchoPAC apparaît dans /Applications).

## Désinstallation

L'interface propose deux niveaux (section « Désinstallation », en bas de page,
avec confirmation) :

- **Désinstaller EchoPAC** — supprime le programme EchoPAC, la copie du média
  (`C:\media`) et les entrées de désinstallation du registre Wine. Le préfixe
  et ses composants (Wine, VC++, SQL Anywhere) sont conservés : une
  réinstallation ne demande que l'étape 5. CLI : `echowinekit uninstall-echopac`.
- **Tout réinitialiser** — supprime en plus le préfixe entier et la
  configuration (les journaux sont conservés dans `~/EchoPAC-WineKit/logs`).
  Repassez ensuite par les étapes 4 puis 5. CLI : `echowinekit reset-kit`.

> ⚠ Dans les deux cas, **les données d'examen stockées dans le préfixe sont
> supprimées** (archive EchoPAC). Sauvegardez-les avant via le bouton
> « Préfixe Windows » (le dossier `drive_c` contient tout : programmes et données).

## Accès aux dossiers

En bas de l'interface, trois boutons ouvrent le Finder directement sur :
- **Dossier du kit** : `~/EchoPAC-WineKit` (configuration `echopac.conf`, journaux) ;
- **Préfixe Windows** : `~/EchoPAC-WineKit/prefix/drive_c` — c'est le « disque C: »
  où EchoPAC est installé (programmes et données) ;
- **Journaux** : `~/EchoPAC-WineKit/logs` (une trace par opération).

> Note licence : ce kit n'installe aucun composant Sentinel/HASP et ne contourne
> aucune protection. Aucun fichier ne peut remplacer légalement la clé USB de
> licence. Si votre version d'EchoPAC l'exige, utilisez la Solution 1
> (dossier `../../Mac-Silicon/` : VM x86 + clé USB redirigée) ou contactez
> GE Healthcare pour une licence adaptée.

## Dépannage

| Symptôme | Action |
|---|---|
| Un indicateur reste gris | Bouton **Diagnostic complet** ; traiter la première ligne `[!!]` |
| L'installeur GE ne s'ouvre pas | Relancer l'étape 5 ; consulter le dernier journal dans `logs/` |
| « setup.exe introuvable » | Vérifier que le média contient `setup.exe` (ou pointez sur son dossier parent) |
| EchoPAC refuse de démarrer (mention de clé/Sentinel) | Votre version exige la licence USB — voir note ci-dessus |
| Performances faibles | `Diagnostic complet` → le renderer doit être `d3dm` (Metal) ; fermer les autres apps |
| Besoin de trace maximale | En CLI : `echowinekit install-echopac --debug <média>` |

### Équivalents en ligne de commande

| Interface | Terminal |
|---|---|
| Carte 1 | `echowinekit install-kit` |
| Carte 2 | `echowinekit create-prefix` |
| Carte 3 | `echowinekit install-echopac "~/Downloads/EchoPac 211.32.1.3"` |
| Carte 4 | `echowinekit launch` / `echowinekit app-bundle` |
| Désinstallation | `echowinekit uninstall-echopac` / `echowinekit reset-kit` |
| Diagnostic | `echowinekit doctor` |

Toutes les commandes acceptent `--kit-root <chemin>` pour installer le kit
ailleurs que dans `~/EchoPAC-WineKit`.
