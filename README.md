# PDF Reader CP 3.3
Programma opensource progettato da Peppe CP con l'aiuto della IA.

## Se vuoi contribuire al mio lavoro ti lascio in basso il pulsante per contribuire anche con una piccola donazione simbolica visto il tempo che ti farò risparmiare per stampare grossi quantitativi di pdf ;-)

[![Donate via PayPal](https://img.shields.io/badge/Donate-PayPal-blue.svg?logo=paypal)](https://paypal.me/PeppeCP)

Lettore PDF a schede in Rust (egui + PDFium): una sola barra in alto con schede e comandi di stampa, documento a tutto schermo.

## Creare l'installer per Windows
**Metodo 1 - sul tuo PC (Windows 10/11 x64):** installa Rust (https://rustup.rs), poi doppio clic su `build.bat`.
Scarica da solo pdfium.dll e Inno Setup e produce `dist\PDFReaderCP_Setup_3.3.exe`.

**Metodo 2 - senza installare nulla:** carica la cartella su GitHub, poi Actions > "Crea installer" > Run workflow.
L'installer si scarica dagli "Artifacts" a fine lavoro; se nel campo `release_tag` scrivi il nome di un tag (per esempio `3.3`)
viene anche allegato alla release di quel tag. Il workflow parte da solo a ogni push su `main` e su un tag `v*` o numerico; pubblicando un ramo con il numero di versione (per esempio `3.3`) crea la release di quella versione con l'installer allegato.

## Uso
- Barra del titolo (una sola riga): **File** | schede (tutto lo spazio disponibile) | icone **Stampa…**, **Stampa subito**, **STAMPA TUTTO** | riduci / massimizza / chiudi.
- Menu **File**: Apri, Chiudi scheda / tutte, Stampa…, Stampa subito, STAMPA TUTTO, Stampante, Pagine per foglio, **Visualizzazione pagine** (mini anteprime / pagine affiancate), Schermo intero, **Info**, Esci.
- **Stampa…** (Ctrl+P): anteprima dettagliata (1, 2, 4, 6, 9, 16 pagine per foglio), copie, fascicola, intervallo, scala, rotazione, centratura, colore con luminosità e contrasto.
- **Stampa subito** (Ctrl+Maiusc+P): scheda corrente con le ultime impostazioni, senza anteprima.
- **STAMPA TUTTO**: tutte le schede aperte, ognuna per intero, in sequenza, con un solo messaggio di conferma. La finestra è più grande e mostra subito **schede, pagine totali, fogli per copia, copie e totale fogli da stampare**.
- **Stampa a gruppo**: si trova nella finestra **STAMPA TUTTO** (sezione "Stampa a gruppo"); togliendo la spunta a **Fascicola** nella finestra di stampa i fogli escono gia' raggruppati. Si scelgono per ogni scheda le pagine da stampare (es. `1` nella prima, `3` nella seconda) e si preme **STAMPA A GRUPPO**: le pagine di file diversi finiscono nello stesso lavoro e, con 2 pagine per foglio (**AFFIANCA 2 PAGINE**), anche sullo stesso foglio.
- **Preferenze del driver**: scegliendo la stampante, colore (colori/bianco e nero), carta e orientamento seguono le preferenze predefinite del driver; nella finestra compare il riepilogo (es. BIANCO E NERO · FRONTE/RETRO).
- Molti PDF: Ctrl+Tab / Ctrl+Maiusc+Tab cambiano scheda, Ctrl+W la chiude, clic con la rotellina sulla scheda la chiude, rotellina del mouse sulla barra scorre le schede.
- **Mini anteprime delle pagine**: colonna a sinistra del documento con tutte le pagine; clic su una miniatura per saltare a quella pagina, si aggiornano mentre scorri. Si mostra o nasconde da **File > Visualizzazione pagine**, dove si possono anche **affiancare 2 pagine** per volta (in quel caso si sfogliano a coppie).
- F11 schermo intero · frecce / PagSu / PagGiu cambiano pagina · Ctrl+rotellina zoom · trascina i PDF nella finestra.
- **Istanza unica**: se il programma e' gia' aperto, un altro PDF (doppio clic, "Apri con", riga di comando) si apre in una nuova scheda della stessa finestra, che torna in primo piano. Usa la porta locale 127.0.0.1:47653.
- La finestra si apre sempre ingrandita. Ogni scheda ha la sua "x" di chiusura integrata (clic con la rotellina = chiudi).
- Stampa...: scala **Adatta alla pagina**, **Dimensioni reali (100%)** oppure **Proporzione personalizzata** (25-200 %); con piu' pagine per foglio la scala e' automatica.
- **Testo selezionabile**: trascina il mouse sul testo della pagina, poi Ctrl+C (o tasto destro > Copia). Ctrl+A seleziona tutto il testo della pagina. I PDF scansionati (solo immagini) non hanno testo da selezionare.
- Con il testo selezionato non si trascina piu' la pagina col mouse: usa rotellina e barre di scorrimento.
- **Poster**: nella finestra di stampa, sezione *Poster*, si sceglie **2 × 2**, **3 × 3** o **4 × 4**: ogni pagina viene ingrandita e divisa su più fogli, con **sovrapposizione** (0-30 mm) per ritagliare e incollare i bordi. Il numero di fogli per copia è indicato nella finestra e nell'anteprima.
- **Cerca aggiornamenti**: nella finestra Info (File > Info) il pulsante controlla l'ultima versione pubblicata e, se ce n'è una più recente, apre la pagina per scaricarla.
- **Stampa...**: pulsante **Impostazioni driver stampante...** (proprieta' del driver: fronte/retro, vassoio, qualita', colore, finitura...). Le scelte del driver vengono usate nella stampa e ricordate per quella stampante; la carta dell'anteprima segue quella del driver.
- **Colore, luminosita' e contrasto**: Colori / Automatico / Bianco e nero (che alla fine è scala di grigi), cursore Scuro-Chiaro e cursore **Contrasto** (meno contrasto = grigi più tenui, più contrasto = neri più decisi, utile per scansioni sbiadite; valgono per anteprima e stampa).
- **Copie**: pulsanti - e + accanto al numero. Con piu' di una copia il programma chiede conferma ("Sicuro di voler stampare N copie del file?", oppure "...dei file aperti?" con STAMPA TUTTO).
- **Fluidita'**: i PDF restano aperti in memoria; durante ridimensionamento e zoom la pagina non viene ridisegnata a ogni passo (si affina appena ti fermi); nell'anteprima di stampa i cursori chiaro/scuro e contrasto e la scelta del colore non ricompongono il foglio.
- **Strumenti** (accanto alle icone di stampa): **Seleziona testo** (T) e **Sposta la pagina con il mouse** (H, trascinando la pagina).
- **Ricerca nel PDF**: campo "Cerca nel PDF" nella barra in basso (Ctrl+F). Invio = risultato successivo, Maiusc+Invio = precedente; i risultati sono evidenziati in giallo (quello corrente in arancione) e la ricerca passa da una pagina all'altra.
- **STAMPA TUTTO**: se sono impostate piu' copie, avvisa (in stampatello) con totale di fogli e chiede se correggere a 1 copia per file (Si), mantenere le impostazioni (No) o annullare.
- **Finestra**: si sposta trascinando qualsiasi spazio libero della barra in alto (anche tra le schede e accanto all'icona); doppio clic sulla barra = ingrandisci/ripristina. Le stampanti sono elencate in ordine alfabetico.
- **Impostazioni** (File > Impostazioni..., Ctrl+,): finestra con due sezioni, Generale (apri sempre in secondo piano, avvia ingrandita, strumento e vista iniziali, **pagine all'avvio**: mini anteprime a sinistra, pagine affiancate oppure come al precedente avvio con salvataggio automatico) e Stampa (stampante, copie, pagine per foglio, colore, conferma con piu' copie). Si salvano con "Salva" nel file settings.json.
- **In secondo piano**: con l'opzione attiva (anche da File > Apri sempre in secondo piano) la finestra parte senza prendere il focus e i PDF aperti da un secondo avvio si aggiungono come schede senza portarla davanti.
- **Intervallo personalizzato**: si possono ripetere le pagine, per esempio `1,1` (pagina 1 due volte, utile con 2 su 1), `2x3` (pagina 2 tre volte) o `1-3x2` (ogni pagina due volte).
- **Schede**: la barra di scorrimento compare sotto le schede solo quando sono troppe.

## Se vuoi contribuire al mio lavoro ti lascio in basso il pulsante per contribuire anche con una piccola donazione simbolica visto il tempo che ti farò risparmiare per stampare grossi quantitativi di pdf ;-)

[![Donate via PayPal](https://img.shields.io/badge/Donate-PayPal-blue.svg?logo=paypal)](https://paypal.me/PeppeCP)

https://github.com/gsepp86/Pdf-Reader-CP
- **STAMPA TUTTO**: passando il mouse sull'icona si vede subito il **totale delle copie** (pagine, fogli per copia, copie e fogli da stampare). La finestra chiede sempre "STAMPARE CON LE IMPOSTAZIONI ATTUALI?" con il riepilogo grande (schede, pagine totali, fogli per copia, copie, totale fogli, stampante, colore), la sezione **Stampa a gruppo** e i pulsanti **STAMPA**, **AFFIANCA 2 PAGINE**, **CAMBIA** (apre le impostazioni di stampa: con **STAMPA TUTTO** stampa tutte le schede con le nuove impostazioni) e **ANNULLA**. Se sono impostate piu' copie compare anche **1 COPIA PER FILE**.
- **Cambio pagina scorrendo**: con la rotella, il touchpad o lo strumento mano, arrivati in fondo alla pagina si continua sulla successiva (e in alto sulla precedente), senza cliccare i pulsanti.
- **Schede tante**: quando non entrano nella barra compaiono piccoli numeri 1, 2, 3... sotto ogni scheda.
- **Finestra di stampa**: la colonna delle impostazioni (compreso il cursore Scuro-Chiaro in "Colore e luminosita'") usa piu' altezza e ha una barra di scorrimento visibile se serve.
- **Stampa in background**: cliccando **Stampa** la finestra si chiude e l'invio parte da solo dopo 2 secondi, cosi' nel frattempo puoi passare a un'altra scheda o avviare altre stampe (i lavori procedono insieme). In basso a destra compaiono un piccolo cerchio di caricamento con l'avanzamento (fogli e schede) e il pulsante **Annulla** per fermare una stampa se qualcosa non va; spariscono quando il lavoro e' in coda nella stampante.
- **Schede spostabili**: trascina una scheda per cambiarne la posizione; STAMPA TUTTO stampa le schede nell'ordine in cui sono disposte.
- **Contorno grigio scuro** attorno alla finestra.
- **Impostazioni di stampa all'avvio** (Impostazioni > Stampa): si sceglie tra *ultime impostazioni utilizzate* e *le mie impostazioni predefinite* (stampante, copie, colore e funzioni del driver come il fronte/retro). All'avvio il programma ripristina sempre la stampa standard (1 pagina per foglio, tutte le pagine, scala adatta, 1 copia) e usa le impostazioni predefinite del driver della stampante, per esempio il fronte/retro se e' impostato come predefinito nel driver (si possono disattivare entrambe le opzioni).
- Il contorno della finestra e' grigio scuro.
- **PDF protetti da password**: all'apertura compare la richiesta della password (con messaggio se e' errata); piu' file protetti si aprono uno dopo l'altro.
- **Finestre spostabili**: stampa, impostazioni, info e le altre finestre si trascinano dove vuoi (si aprono al centro la prima volta).
