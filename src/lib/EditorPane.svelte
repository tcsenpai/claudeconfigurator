<script lang="ts">
  import { onDestroy, untrack } from "svelte";
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

  // Build the editor once on mount. CodeMirror owns the content after that, so
  // we must NOT re-run on `doc`/`onChange` changes (that would destroy and
  // rebuild the editor on every keystroke, losing cursor/selection/history).
  // The parent remounts this component per file via {#key path}, so a file
  // switch gets a fresh editor for free.
  $effect(() => {
    // untrack: read props once at build time, do not subscribe to their
    // changes (esp. `doc`, which the parent updates on every keystroke).
    view = untrack(() => makeEditor({ parent: el, doc, lang, dir, onFollow, onChange }));
    return () => view?.destroy();
  });

  onDestroy(() => view?.destroy());
</script>

<div class="pane" bind:this={el}></div>

<style>
  .pane { height: 100%; overflow: hidden; }
  :global(.cm-editor) { height: 100%; }
</style>
