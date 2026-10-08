# <img src="public/pokko.svg" alt="pokko" height="42" align="absmiddle"> pokko

[![Build](https://github.com/browzomje/pokko/actions/workflows/build.yml/badge.svg)](https://github.com/browzomje/pokko/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/browzomje/pokko)](https://github.com/browzomje/pokko/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

App desktop **Rust + Tauri 2 + Svelte 5**, con navigazione nel catalogo, coda download, laboratorio delle fonti e scorciatoie. Ricerca e download da fonti italiane e inglesi configurabili.

## Installazione

Scarica un pacchetto dalla [ultima release](https://github.com/browzomje/pokko/releases/latest).

| Piattaforma | Pacchetto |
| --- | --- |
| Windows x86_64 | `.exe` (NSIS) oppure `.msi` |
| Debian / Ubuntu | `.deb` |
| Fedora / openSUSE | `.rpm` |
| Linux x86_64 portabile | `.AppImage` (rendilo eseguibile prima di avviarlo) |

## Avvio da sorgente

```sh
git clone git@github.com:browzomje/pokko.git
cd pokko
sh run_gui.sh
# Versione ottimizzata:
sh run_gui.sh --release
```

Servono Node.js e Rust; su Linux anche GTK 3, WebKitGTK 4.1 e librsvg. Lo script installa le dipendenze frontend se mancanti e sceglie una porta libera. `npm run dev` mostra solo l’interfaccia nel browser; ricerca, preferenze e download richiedono Tauri.

La cartella **Download del sistema operativo** è preselezionata. Eventuali percorsi personalizzati della versione precedente vengono conservati. Le preferenze si salvano automaticamente dopo una breve pausa nella modifica; lo stato di salvataggio e gli errori di validazione sono visibili nelle impostazioni.

1. Cerca un titolo in tutte le fonti attive, oppure scegli una fonte.
2. Scegli Tutti, Manga o Comics. Ogni copertina mostra fonte, lingua e autori quando disponibili.
3. Apri una copertina restando in Esplora; **Torna ai risultati** conserva ricerca e pagina. Clicca sul nome del volume o sulla checkbox per selezionarlo; il pulsante a freccia mostra i capitoli.
4. Scarica un capitolo, un volume, i selezionati o tutti: PDF è il formato predefinito, modificabile in **Impostazioni**. Il download parte direttamente; rimani sul dettaglio in Esplora. Segui avanzamento e verifica delle pagine in **Downloads** (`Ctrl+J`).
5. Usa pausa/ripresa oppure il cestino rosso per eliminare un lavoro dalla lista. Le scan già salvate rimangono sul disco.

La finestra parte a 1360×940, con minimo 1120×800.

## Fonti

In **Fonti del catalogo**, le fonti sono disposte in due colonne (una nelle finestre strette). Puoi abilitarle, rimuoverle o aprire **Modifica** e **Laboratorio**. **Aggiungi fonte** apre la sezione **Laboratorio fonti** con una bozza precompilata: la fonte entra nel catalogo solo dopo **Salva fonte**. I preset incorporati sono in `resources/sources.json`; le modifiche personali vengono salvate nella directory dati dell’app.

| Fonte | Adattatore / verifica |
| --- | --- |
| MangaWorld .mx | Catalogo e gruppi di volumi verificati; IT |
| MangaDex | API: ricerca IT/EN, feed paginato, volumi e MangaDex@Home |
| MangaFreak | HTML verificato; EN, elenco capitoli e immagini |
| MangaRead | HTML verificato; EN, ricerca, capitoli e immagini |
| ReadComicsOnline | Adattatore comics; EN, ricerca, raccolte TPB, albi e payload statico completo delle pagine |

Catalogo aggiornato il 6 ottobre 2026. I preset non funzionanti e i mirror duplicati vengono rimossi anche dalle preferenze esistenti al primo avvio della nuova versione. Le fonti personalizzate restano disponibili. Puoi aggiungere e modificare fonti; fonti e risultati sono ordinati con IT prima di EN. Gli errori di una fonte non nascondono i risultati delle altre. La ricerca dà priorità ai termini presenti nel titolo e negli autori pubblicati dalla fonte, anche con un cognome parziale (es. `invincible kirk` per Robert Kirkman). Se la ricerca completa non restituisce risultati nella prima pagina, prova una sola ricerca aggiuntiva senza l’ultima parola e consulta fino a tre schede per acquisire gli autori mancanti. Le fonti che non pubblicano autori non possono garantire questa distinzione.

**HTML generico** supporta URL di ricerca con `{query}` e `{page}`, selettori per risultati, capitoli, descrizione, pagina successiva e immagini del lettore. Legge anche liste di immagini in payload JSON statici (`images`, `pages`, `chapterImages`). Non esegue JavaScript, non aggira CAPTCHA, verifiche o certificati non validi. Lettori dinamici e API specifiche possono richiedere un adattatore dedicato. Per fonti HTML la lingua è quella configurata: imposta un dominio o un percorso ricerca che restituisca solo IT oppure EN.

In **Esplora → Comics** puoi cercare serie occidentali oppure incollare un link HTTPS di una serie da una fonte attiva. L’app legge gli albi e scarica tutte le pagine nella propria coda: non apre lettori esterni. ReadComicsOnline presenta ogni raccolta TPB come un volume e ogni singolo albo come un elemento separato; se la fonte non offre raccolte, l’app non inventa raggruppamenti editoriali. Le fonti HTML personalizzate possono essere assegnate al catalogo Manga o Comics. Il parser comics usa i dati JSON statici della pagina e rifiuta sequenze con numeri mancanti o duplicati.

Quando una fonte non pubblica gruppi di volumi, i capitoli vengono mostrati singolarmente, senza inventare volumi. La GUI segnala quando il raggruppamento non è disponibile. MangaDex può mantenere un titolo nel catalogo anche dopo la rimozione delle sue scan; l’app segnala quando non ci sono capitoli scaricabili nella lingua scelta. Per capitoli duplicati di MangaDex viene scelta una sola versione nella lingua richiesta.

## Esplora e organizzazione dei file

**Esplora** include Tutti, Manga e Comics. La ricerca pubblica progressivamente i risultati di ogni fonte prima di attendere i metadati aggiuntivi: le schede sono subito cliccabili. Aggiornamenti dei creatori riordinano l’elenco per titolo e autore senza duplicare le schede. Le fonti vengono interrogate in parallelo (fino a otto); le richieste a ciascun dominio rispettano il suo intervallo minimo. Ogni ricerca ha un identificatore per ignorare eventi estranei. Aprendo una copertina il dettaglio resta dentro Esplora, senza schede aggiuntive nella sidebar o in alto. **Torna ai risultati** conserva ricerca, pagina e posizione di scorrimento. Le fonti sotto le copertine distinguono le diverse edizioni; gli eventuali errori sono raccolti in una disclosure compatta.

I volumi indicati dalla fonte restano espandibili; i capitoli senza volume sono selezionabili direttamente. **Organizza i file** consente un file per volume/albo o capitolo disponibile, un file per capitolo anche nei volumi raggruppati, oppure un unico file per la selezione. L’anteprima mostra il numero e i nomi dei file della selezione; il numero corrisponde ai lavori inviati alla coda. Una raccolta esposta come unico lettore non può essere divisa in capitoli editoriali assenti nella fonte: in quel caso l’opzione per capitolo è disabilitata. I nomi delle selezioni unite includono un identificatore, così selezioni diverse con gli stessi estremi non vengono confuse. Il formato resta quello delle Impostazioni (PDF, CBZ, CBR o foto).

## Laboratorio delle fonti

1. Apri **Laboratorio** dalla fonte o da **Aggiungi fonte** o dalla voce nella sidebar. La configurazione rimane una bozza fino a **Salva fonte**.
2. Segui **Sito → Ricerca → Serie → Pagine → Verifica e salva**. Un nuovo connettore parte con URL, modello e selettori vuoti: nel passo Sito inserisci la homepage, poi nel passo Ricerca copia un URL ottenuto realmente cercando sul sito. I campi cambiano con il passo, mostrando solo quelli pertinenti. Ogni campo ha un’icona e un aiuto con esempio. Parti dall’URL dei risultati di una ricerca sul sito, non da una singola immagine. Inserisci anche il titolo cercato: il laboratorio ricava il modello `{query}` dall’URL. Il modello salvato può generare URL per altri titoli; il solo dominio non permette di ricavarlo. I moduli POST e le ricerche gestite esclusivamente da JavaScript richiedono un adattatore dedicato. Premi **Acquisisci pagina** per salvare la pagina osservata.
3. Puoi includere CSS e immagini nel campione: fino a 300 risorse, 12 MB per risorsa e 64 MB per campione, con i limiti globali e della fonte. Il campione viene salvato nella directory dati dell’app, sotto `connector-samples`. Acquisisci le tre pagine separatamente per provare l’intero flusso.
4. Clicca nell’anteprima per ottenere un selettore, HTML e proprietà CSS dell’elemento. **Elemento padre** risale nel DOM; scegli il campo di destinazione e **Usa selettore**, oppure **Usa percorso esatto** per il singolo elemento. La scheda HTML permette modifiche di lavoro per le prove correnti.
5. Attiva **Registra percorso**: clicca il link di una serie nei risultati, poi il link di un capitolo e infine una scan. Il laboratorio salva selettori e URL dei tre passi nella bozza. **Naviga nei campioni offline** riapre pagine già salvate; se manca una pagina, acquisiscila o importala. Disattiva l’opzione per acquisire automaticamente le pagine seguite. Il percorso salvato configura il connettore; il downloader usa i selettori per qualunque titolo, senza ripetere i link di prova. **Prova estrazione** usa lo stesso parser dei download reali e mostra titoli, URL, gruppi, descrizione e pagina successiva. Puoi usare un risultato come URL del prossimo campione. Le modifiche alla configurazione invalidano il risultato precedente.
6. Sul campione del lettore, **Scarica PDF di prova** crea un PDF con tutte le immagini estratte e verifica il numero di pagine. Il test offline usa solo risorse salvate e segnala quelle mancanti prima di creare file; il test online scarica quelle mancanti. Ogni prova usa una cartella nuova; acquisizioni e download di prova sono annullabili.
7. Le due paginazioni sono distinte: **Pagina successiva dei risultati** serve alla ricerca, **Pagina successiva delle scan** segue altre pagine web dello stesso capitolo. Se il modello ricerca non contiene `{page}`, vengono seguiti i link successivi. Il test PDF segue anche i campioni delle pagine successive e segnala quelli mancanti offline; cicli e sequenze superiori a 100 pagine HTML vengono rifiutati.
8. Importa HTML UTF-8 (indicando l’URL originale) o importa/esporta un campione JSON con HTML originale, configurazione e risorse. Le modifiche temporanee all’HTML non sovrascrivono il campione originale. I campioni salvati sono selezionabili ed eliminabili dal laboratorio.

L’anteprima è una copia statica offline con immagini e CSS acquisiti: il sito non esegue script e l’anteprima non effettua richieste remote. Non è una copia completa del sito e non riproduce lettori che richiedono JavaScript. CSS importati, font e altre risorse non acquisite possono alterare l’aspetto; gli URL estratti restano quelli originali. I selettori sono modificabili per l’adattatore HTML generico; gli adattatori dedicati possono essere provati ma usano il loro parser. MangaDex usa l’API e dispone di **Verifica connessione API** nel passo Sito. La fonte è disattivata per impostazione predefinita e nelle preferenze migrate, perché nell’ambiente attuale il dominio risolve su indirizzi locali e presenta un certificato incompatibile. Non vengono ignorati i controlli HTTPS. Può essere riabilitata dopo un test riuscito.

Nel passo finale un connettore HTML generico può essere abilitato dopo prove riuscite di ricerca, capitoli e immagini. Una modifica ai campi pertinenti invalida la prova; è sempre possibile salvare una bozza disabilitata.

La ricerca usa anche i creatori pubblicati dalla fonte: metadati JSON-LD, meta autore e selettore configurato. Per ricerche di più parole arricchisce al massimo tre candidati per fonte, privilegiando titoli più pertinenti e brevi; conserva in cache fino a 512 schede, rispettando gli intervalli della fonte. Invincible (2003) pubblica Robert Kirkman e Cory Walker nei suoi metadati. Non serve un database esterno per questo caso; fonti prive di crediti non ricevono autori inventati.

Gli errori di rete ora includono la causa sottostante e suggerimenti per timeout, DNS e certificati. Un certificato non valido non viene ignorato: occorre correggere la connessione o la risoluzione del dominio.

## Downloads e limiti delle richieste

I completati lasciano automaticamente la vista **Coda** e passano nella **Cronologia**, con righe compatte. **Pulisci cronologia** li archivia: **Mostra archiviati** li rende nuovamente visibili. Questa operazione conserva file, metadati di verifica e riconoscimento dei duplicati. Se una verifica all’avvio richiede una riparazione, il lavoro torna visibile nella coda.

- Una coda sequenziale: la pausa interrompe in modo riprendibile il lavoro e lascia partire il successivo. Eliminazione singola o di tutti i lavori incompleti; apertura della cartella con il pulsante dedicato.
- Ricerca su fino a tre fonti indipendenti in parallelo, con limiti per dominio condivisi con i download.
- Intervallo globale predefinito 0,5 secondi (applicato una volta anche alle preferenze esistenti); il minimo della fonte viene sempre rispettato (preset HTML: 1 secondo; MangaDex API: 0,5 secondi).
- HTTP 403/429 sospende i lavori successivi della stessa fonte. `Retry-After` viene rispettato, anche in formato data; in sua assenza si applica un cooldown di 60 secondi.
- Ogni pagina mancante o danneggiata viene ritentata automaticamente fino a tre volte, con attesa crescente. HTTP 403/429 e pausa/annullamento fermano i tentativi. Dopo un errore persistente, Riprendi verifica e riutilizza le pagine sane.
- Elenco delle pagine dei lettori HTML salvato in cache: la ripresa evita di interrogare nuovamente i capitoli già estratti. MangaDex aggiorna sempre il suo server immagini.
- Le scan vengono decodificate per intero con limiti di memoria, e controllate con dimensione e CRC32 rispetto ai metadati salvati. Una semplice intestazione JPEG/PNG non è sufficiente per considerare valida una pagina. Il conteggio finale deve coincidere con tutte le pagine elencate dalla fonte; non può rilevare pagine che la fonte stessa omette senza indicarle.
- Scritture delle immagini, CBZ, CBR e PDF con file temporaneo e rinomina al completamento.
- Nuovi download separati per titolo, lingua e fonte: `Manga [en]/Fonte/Volume/0001-Capitolo/0001.jpg`. I lavori precedenti conservano il percorso originale.
- Esportazione esclusiva: foto singole, CBZ, CBR oppure PDF. **CBZ** è ZIP; **CBR** è un vero RAR5 in modalità stored (immagini già compresse), senza dipendenze da WinRAR o comandi esterni. PDF converte JPG/PNG/WebP/GIF in pagine JPEG a qualità 95. I formati non decodificabili non ricevono l’etichetta Verificato. I temporanei vengono rimossi dopo la verifica del file esportato.
- Le etichette distinguono verifica in corso, riparazione automatica, verifica incompleta e file verificato, con conteggio delle pagine e delle riparazioni. Gli archivi CBZ/CBR vengono riaperti e verificati; per PDF si verifica anche il numero di pagine.
- Ogni esportazione ha un file `.integrity.json` con dimensione, CRC32 e pagine attese. I download completati e verificati vengono ricontrollati all’avvio: file mancanti o modificati vengono rimessi automaticamente in coda. Se l’archivio finale è corrotto e i temporanei sono stati rimossi, viene ricreato il volume; la ripresa di un download incompleto riscarica solo le pagine mancanti o corrotte. Conserva i metadati accanto ai file.
- Il cestino interrompe anche i trasferimenti in corso e rimuove il lavoro dalla coda persistente. I vecchi lavori “Annullato” vengono ripuliti al primo avvio della nuova versione.

L’intervallo limita il carico ma non garantisce l’assenza di blocchi. Dopo un riavvio, i lavori incompleti sono in pausa; premi Riprendi per continuarli. La coda e le preferenze sono in `queue.json` nella directory dati di Tauri.

## Scorciatoie

| Azione | Tasti |
| --- | --- |
| Esplora / cerca | Ctrl+K |
| Torna ai risultati | Ctrl+← oppure Esc |
| Downloads | Ctrl+J |
| Fonti del catalogo | Ctrl+Shift+F |
| Laboratorio fonti | Ctrl+Shift+L |
| Cambia tema | Ctrl+Shift+T |
| Scorciatoie | Ctrl+Shift+H |
| Impostazioni | Ctrl+, |
| Chiudi dialog / messaggi | Esc |

Su macOS sono disponibili anche le combinazioni con Cmd.

## Build e verifiche

```sh
npm run check
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build -- --no-bundle
# Pacchetti desktop:
npm run tauri build
```

La fixture browser `tests/ui-fixture.html` (aprila con Vite in esecuzione) usa un backend simulato e dati locali, senza modificare preferenze o download reali. Permette di verificare ricerca, dettaglio, ritorno ai risultati, organizzazione dei file e ispettore offline. `npm test` verifica i piani dei download; i test Rust del laboratorio provano i parser e la creazione completa di PDF dalle risorse locali.

I test normali usano fixture HTML locali e un server HTTP locale per verificare parser, JSON, cancellazione, persistenza, migrazione, cooldown, riparazione delle pagine, conteggio dei PDF, CRC degli archivi e riconoscimento dei file danneggiati dopo il completamento. I test live sono esclusi dalla CI:

```sh
# PDF comics completo di 27 pagine:
cargo test --manifest-path src-tauri/Cargo.toml live_comics_complete_pdf -- --ignored --nocapture
# Download di 23 scan MangaWorld, CBZ e ripresa senza riscaricare:
cargo test --manifest-path src-tauri/Cargo.toml live_download_cbz_and_resume -- --ignored --nocapture
# Ricerca, dettaglio, estrazione e un’immagine valida da MangaDex e MangaFreak:
cargo test --manifest-path src-tauri/Cargo.toml live_multiple_sources -- --ignored --nocapture
```

## GitHub Actions

Ogni push e pull request esegue controlli Svelte, test Rust, rustfmt e Clippy, poi compila gli installer **Linux (DEB, RPM, AppImage)** e **Windows (NSIS, MSI)**. I pacchetti sono disponibili come artifact nella pagina del run per 14 giorni. È disponibile anche l’avvio manuale con `workflow_dispatch`.


### Release automatiche

Ogni push su `main` esegue semantic-release: `fix:` / `perf:` / `refactor:` aumentano la patch, `feat:` la minor e `BREAKING CHANGE` la major. `build(release):` permette una patch per modifiche al packaging. La prima release è `1.0.0`.

Le emoji si inseriscono dopo il prefisso, ad esempio `feat: ✨ nuova funzione` o `fix: 🐛 correzione`. Le note di release raggruppano le modifiche con icone.

Versioni sincronizzate automaticamente in npm, Tauri, Cargo e relativi lockfile, changelog e tag `vX.Y.Z`. La release resta in bozza finché le build Linux e Windows non passano e sono presenti tutti gli installer: `.exe` (NSIS), `.msi`, `.deb`, `.rpm`, `.AppImage`, più `SHA256SUMS.txt`. I pacchetti sono x86_64. Non è richiesto un token personale: basta il `GITHUB_TOKEN` del workflow con permesso contents/write.

Se una build fallisce, riavvia i job falliti dalla pagina Actions oppure avvia **Release (semantic-release)** indicando il tag della bozza nel campo `release_tag`. I commit senza bump eseguono i controlli e le build senza pubblicare una versione. AppImage offre il pacchetto Linux portabile; Flatpak non è incluso.

### Pagine, copertine e velocità

MangaDex fornisce il numero di pagine nel feed quando disponibile. Per gli altri lettori il pulsante **Conta pagine** legge solo l’elenco delle scan: non scarica immagini, ma ogni capitolo richiede una richiesta. Il totale del volume viene mostrato solo quando tutti i capitoli sono noti. Gli elenchi HTML contati vengono riusati durante il download.

Le scan su CDN separati dal lettore usano un intervallo di 500 ms; le pagine HTML mantengono il massimo tra intervallo globale e minimo della fonte. Non si aggiunge più l’intera durata della risposta all’attesa successiva. Il profilo rapido seleziona 500 ms globali, mantenendo i minimi delle fonti. HTTP 403/429 e Retry-After sospendono le richieste; non si aggirano i blocchi. Il trasferimento resta sequenziale e la velocità dipende anche dal server e dalla dimensione delle scan.

Le copertine MangaDex vengono caricate nel desktop tramite Rust anziché direttamente dal WebView; immagini non disponibili mostrano una scheda con il titolo. Le serie comics si cercano direttamente in Esplora; nella vista Comics è disponibile anche l’apertura da un link di una fonte attiva.

## Organizzazione del frontend

`App.svelte` coordina stato, navigazione e operazioni. Le schermate sono in `src/pages`; sidebar, scorciatoie, schede download, browser dei campioni, ispezione e guida sono in `src/components`. La pianificazione delle esportazioni, il ranking, i modelli delle fonti e l’accesso IPC sono funzioni separate. Il laboratorio conserva una bozza esplicita fino al salvataggio.

## AI Disclosure

This project was developed with the assistance of Large Language Models, used to support code writing and documentation.

## Licenza

[MIT](LICENSE) · Copyright © 2026 browzomje.
