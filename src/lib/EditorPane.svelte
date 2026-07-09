<script lang="ts">
  import { onDestroy } from "svelte";
  import { EditorView } from "@codemirror/view";
  import { makeEditor } from "./editor";

  interface Props {
    doc: string;
    dir?: string;
    lang?: "markdown" | "json";
    onFollow?: (target: string) => void;
    onChange?: (text: string) => void;
  }
  let { doc, dir = "", lang = "markdown", onFollow = () => {}, onChange = () => {} }: Props = $props();

  let el: HTMLDivElement;
  let view: EditorView | undefined;

  // Rebuild the editor whenever the loaded doc identity changes.
  $effect(() => {
    // reference doc + lang + dir so the effect re-runs on file switch
    doc; lang; dir;
    view?.destroy();
    view = makeEditor({ parent: el, doc, lang, dir, onFollow, onChange });
    return () => view?.destroy();
  });

  onDestroy(() => view?.destroy());
</script>

<div class="pane" bind:this={el}></div>

<style>
  .pane { height: 100%; overflow: hidden; }
  :global(.cm-editor) { height: 100%; }
</style>
