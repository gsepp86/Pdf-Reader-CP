# PDF Reader CP 2.2
Programma opensource progettato da Peppe CP con l'aiuto della IA.

Lettore PDF a schede in Rust (egui + PDFium): una sola barra in alto con schede e comandi di stampa, documento a tutto schermo.

## Se vuoi contribuire al mio lavoro ti lascio in basso il pulsante per contribuire anche con una piccola donazione simbolica visto il tempo che ti farò risparmiare per stampare grossi quantitativi di pdf ;-)

[![Donate via PayPal](https://img.shields.io/badge/Donate-PayPal-blue.svg?logo=paypal)](https://paypal.me/PeppeCP)

## Creare l'installer per Windows
**Metodo 1 - sul tuo PC (Windows 10/11 x64):** installa Rust (https://rustup.rs), poi doppio clic su `build.bat`.
Scarica da solo pdfium.dll e Inno Setup e produce `dist\PDFReaderCP_Setup_2.2.exe`.

**Metodo 2 - senza installare nulla:** carica la cartella su GitHub, poi Actions > "Crea installer" > Run workflow.
L'installer si scarica dagli "Artifacts" a fine lavoro.

## Uso
- Barra del titolo (una sola riga): **File** | schede (tutto lo spazio disponibile) | icone **Stampa…**, **Stampa subito**, **STAMPA TUTTO** | riduci / massimizza / chiudi.
- Menu **File**: Apri, Chiudi scheda / tutte, Stampa…, Stampa subito, STAMPA TUTTO, Stampante, Pagine per foglio, Schermo intero, **Info**, Esci.
- **Stampa…** (Ctrl+P): anteprima dettagliata (1, 2, 4, 6, 9, 16 pagine per foglio), copie, fascicola, intervallo, scala, rotazione, centratura.
- **Stampa subito** (Ctrl+Maiusc+P): scheda corrente con le ultime impostazioni, senza anteprima.
- **STAMPA TUTTO**: tutte le schede aperte, ognuna per intero, in sequenza, con un solo messaggio di conferma.
- Molti PDF: Ctrl+Tab / Ctrl+Maiusc+Tab cambiano scheda, Ctrl+W la chiude, clic con la rotellina sulla scheda la chiude, rotellina del mouse sulla barra scorre le schede.
- F11 schermo intero · frecce / PagSu / PagGiu cambiano pagina · Ctrl+rotellina zoom · trascina i PDF nella finestra.
- **Istanza unica**: se il programma e' gia' aperto, un altro PDF (doppio clic, "Apri con", riga di comando) si apre in una nuova scheda della stessa finestra, che torna in primo piano. Usa la porta locale 127.0.0.1:47653.
- La finestra si apre sempre ingrandita. Ogni scheda ha la sua "x" di chiusura integrata (clic con la rotellina = chiudi).
- Stampa...: scala **Adatta alla pagina**, **Dimensioni reali (100%)** oppure **Proporzione personalizzata** (25-200 %); con piu' pagine per foglio la scala e' automatica.
- **Testo selezionabile**: trascina il mouse sul testo della pagina, poi Ctrl+C (o tasto destro > Copia). Ctrl+A seleziona tutto il testo della pagina. I PDF scansionati (solo immagini) non hanno testo da selezionare.
- Con il testo selezionato non si trascina piu' la pagina col mouse: usa rotellina e barre di scorrimento.
- **Stampa...**: pulsante **Impostazioni driver stampante...** (proprieta' del driver: fronte/retro, vassoio, qualita', colore, finitura...). Le scelte del driver vengono usate nella stampa e ricordate per quella stampante; la carta dell'anteprima segue quella del driver.
- **Colore e luminosita'**: Colori / Scala di grigi / Bianco e nero e cursore Scuro-Chiaro (valido per anteprima e stampa).
- **Copie**: pulsanti - e + accanto al numero. Con piu' di una copia il programma chiede conferma ("Sicuro di voler stampare N copie del file?", oppure "...dei file aperti?" con STAMPA TUTTO).
- **Fluidita'**: i PDF restano aperti in memoria; durante ridimensionamento e zoom la pagina non viene ridisegnata a ogni passo (si affina appena ti fermi); nell'anteprima di stampa il cursore chiaro/scuro e la scelta del colore non ricompongono il foglio.
- **Strumenti** (accanto alle icone di stampa): **Seleziona testo** (T) e **Sposta la pagina con il mouse** (H, trascinando la pagina).
- **Ricerca nel PDF**: campo "Cerca nel PDF" nella barra in basso (Ctrl+F). Invio = risultato successivo, Maiusc+Invio = precedente; i risultati sono evidenziati in giallo (quello corrente in arancione) e la ricerca passa da una pagina all'altra.
- **STAMPA TUTTO**: se sono impostate piu' copie, avvisa (in stampatello) con totale di fogli e chiede se correggere a 1 copia per file (Si), mantenere le impostazioni (No) o annullare.
- **Finestra**: si sposta trascinando qualsiasi spazio libero della barra in alto (anche tra le schede e accanto all'icona); doppio clic sulla barra = ingrandisci/ripristina. Le stampanti sono elencate in ordine alfabetico.
- **Impostazioni** (File > Impostazioni..., Ctrl+,): finestra con due sezioni, Generale (apri sempre in secondo piano, avvia ingrandita, strumento e vista iniziali) e Stampa (stampante, copie, pagine per foglio, colore, conferma con piu' copie). Si salvano con "Salva" nel file settings.json.
- **In secondo piano**: con l'opzione attiva (anche da File > Apri sempre in secondo piano) la finestra parte senza prendere il focus e i PDF aperti da un secondo avvio si aggiungono come schede senza portarla davanti.
- **Intervallo personalizzato**: si possono ripetere le pagine, per esempio `1,1` (pagina 1 due volte, utile con 2 su 1), `2x3` (pagina 2 tre volte) o `1-3x2` (ogni pagina due volte).
- **Schede**: la barra di scorrimento compare sotto le schede solo quando sono troppe.

