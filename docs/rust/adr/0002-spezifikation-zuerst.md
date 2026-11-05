# ADR-0002: Spezifikation zuerst — als ausführbare Testvektoren

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Die Frage war: „nur die Funktionen aufschreiben und bei Rust von vorn beginnen?" Funktionen aufzuschreiben ist der
leichte Teil. Was beim Neuschreiben verloren geht, ist das Wissen darüber, was schiefgehen kann. Die Durchsicht von
1.1.2 fand es dort, wo keine Funktion steht: ein abgelehnter Wert, der als „bestanden" verbucht wird; eine Fehlerzeile
über der Lesegrenze; Kerne, die nach einem Abbruch auf 0 bleiben; ein 24-Stunden-Umbruch. Heute steht dieses Wissen in
453 Testfällen und in Kommentaren.

## Entscheidung

1. **Testvektoren statt Prosa.** Für jeden Baustein mit reiner Logik entsteht eine JSON-Datei mit Eingaben und
   erwarteten Ausgaben, unter `spec/vectors/`: Empfehlung, Auto-Durchlauf, Schätzer, Tuner, Gedächtnis, Tiefe,
   Kernzuordnung, APIC-Zuordnung, Log-Lesen, Download-Regeln, Zeitformat, Absturz-Einordnung.
2. **C# führt sie zuerst aus** (Gate G1). Ein Vektor, den C# nicht besteht, ist ein Fehler in C# oder im Vektor, und
   beides will man *vor* dem Neubau wissen.
3. **Rust muss dieselben Vektoren bestehen** (Gate G2). Das ist der Paritätsnachweis.
4. **Jede Invariante (I1–I14 im Plan) braucht mindestens einen Vektor.** Eine Invariante ohne Vektor gilt als
   nicht gesichert.
5. **Zwei Arten von Vektoren.** `parity`: beschreibt, was C# heute tut; C# muss ihn bestehen (Gate G1), Rust ebenso. `new`:
   beschreibt Verhalten, das erst die Rust-Fassung bekommt (Absturzbehandlung, Protokollregel, „gestört", flache Planform);
   nur Rust muss ihn bestehen (Gate G2). Ohne die Trennung würde „C# besteht alle Vektoren" das neue Verhalten verbieten.
6. **Die Produktspezifikation** (ein Plan in drei Tiefen, eine Laufzeit über dem Startknopf, Tooltips statt Absätzen, eine
   Empfehlung statt vierzehn Profilen) steht in `spec/ux.md`. Sie ist das Produkt; die Sprache ist es nicht.

## Alternativen

- **Nur Text.** Verworfen: nicht prüfbar, veraltet, wird beim Bauen nicht gelesen.
- **Nur die C#-Tests nach Rust abschreiben.** Verworfen: doppelte Pflege, Auseinanderlaufen. Gemeinsame Vektoren
  zwingen beide Seiten auf dieselbe Wahrheit.

## Folgen

- Etappe E1 kostet Zeit, bevor eine Zeile Rust steht. Sie ist die billigste Absicherung gegen Regression.
- Vektoren decken nur reine Logik. Hardwareverhalten braucht die Hardwareprobe ([ADR-0012](0012-pruefstrategie.md)).

## Prüfung

Gate **G1**: C# besteht alle `parity`-Vektoren, und die Liste der Invarianten ohne Vektor ist leer.
