<script lang="ts">
  import { Search, BookOpen, Images, Globe, Check } from "lucide-svelte";
  let { kind, onStep }: { kind: string; onStep: (kind: string) => void } =
    $props();
  const steps = [
    {
      value: "setup",
      label: "1. Sito",
      icon: Globe,
      text: "Inserisci il sito che hai trovato, scegli nome, catalogo e lingua. Per un sito nuovo usa HTML generico. Il dominio identifica il sito; il percorso della ricerca verrà imparato nel prossimo passo.",
    },
    {
      value: "search",
      label: "2. Ricerca",
      icon: Search,
      text: "Esegui una ricerca sul sito. Incolla qui l’URL dei risultati e il titolo cercato. Registra il link di una copertina: diventa il selettore risultati.",
    },
    {
      value: "detail",
      label: "3. Serie",
      icon: BookOpen,
      text: "Questa pagina contiene l’elenco dei capitoli o degli albi. Registra un link del capitolo: diventa il selettore capitoli.",
    },
    {
      value: "reader",
      label: "4. Pagine",
      icon: Images,
      text: "Apri il lettore di un capitolo. Registra una scansione: diventa il selettore immagini. Prova l’estrazione, controlla tutte le immagini e scarica un PDF di prova.",
    },
    {
      value: "review",
      label: "5. Verifica e salva",
      icon: Check,
      text: "Controlla che ricerca, capitoli e immagini siano stati estratti. Puoi salvare una bozza disabilitata in qualsiasi momento; abilita la fonte dopo aver verificato tutti e tre i passi.",
    },
  ];
</script>

<div class="lab-guide">
  <div class="lab-tabs">
    {#each steps as step}<button
        class:chosen={kind === step.value}
        onclick={() => onStep(step.value)}
        ><step.icon size={15} />{step.label}</button
      >{/each}
  </div>
  <p>{steps.find((s) => s.value === kind)?.text}</p>
  <small
    ><Globe size={13} /> Le frecce “pagina successiva” sono due campi distinti: risultati
    di ricerca e pagine dello stesso capitolo. Non usare il link al capitolo seguente
    come pagina del lettore.</small
  >
</div>
