import { EditorState, StateEffect, StateField, RangeSetBuilder } from "@codemirror/state";
import { EditorView, keymap, Decoration, type DecorationSet } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { json } from "@codemirror/lang-json";
import { scanRefs, type Ref } from "./api";

/** Effect carrying freshly-scanned @-refs to the decoration field. */
const setRefs = StateEffect.define<Ref[]>();

const resolvedMark = Decoration.mark({ class: "cm-ref cm-ref-ok" });
const missingMark = Decoration.mark({ class: "cm-ref cm-ref-missing" });

const refField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    deco = deco.map(tr.changes);
    for (const e of tr.effects) {
      if (e.is(setRefs)) {
        const b = new RangeSetBuilder<Decoration>();
        for (const r of e.value) {
          b.add(r.start, r.end, r.target ? resolvedMark : missingMark);
        }
        deco = b.finish();
      }
    }
    return deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});

/**
 * Editor factory.
 * @param onFollow called when a resolved @-ref is ctrl/cmd-clicked.
 * @param onChange called (debounced) with the current doc text.
 */
export function makeEditor(opts: {
  parent: HTMLElement;
  doc: string;
  lang: "markdown" | "json";
  dir: string;
  onFollow: (targetPath: string) => void;
  onChange: (text: string) => void;
}): EditorView {
  let refs: Ref[] = [];
  let scanTimer: number | undefined;
  let changeTimer: number | undefined;

  const rescan = (view: EditorView) => {
    if (opts.lang !== "markdown") return;
    clearTimeout(scanTimer);
    scanTimer = setTimeout(async () => {
      refs = await scanRefs(view.state.doc.toString(), opts.dir);
      view.dispatch({ effects: setRefs.of(refs) });
    }, 250) as unknown as number;
  };

  const refAt = (pos: number): Ref | undefined =>
    refs.find((r) => pos >= r.start && pos <= r.end && r.target);

  const view = new EditorView({
    parent: opts.parent,
    state: EditorState.create({
      doc: opts.doc,
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        opts.lang === "json" ? json() : markdown(),
        refField,
        EditorView.updateListener.of((u) => {
          if (u.docChanged) {
            rescan(u.view);
            clearTimeout(changeTimer);
            changeTimer = setTimeout(
              () => opts.onChange(u.state.doc.toString()),
              200,
            ) as unknown as number;
          }
        }),
        EditorView.domEventHandlers({
          mousedown(ev, view) {
            if (!(ev.metaKey || ev.ctrlKey)) return;
            const pos = view.posAtCoords({ x: ev.clientX, y: ev.clientY });
            if (pos == null) return;
            const r = refAt(pos);
            if (r?.target) {
              ev.preventDefault();
              opts.onFollow(r.target);
            }
          },
        }),
        EditorView.theme({
          "&": { height: "100%", fontSize: "13px" },
          ".cm-scroller": { fontFamily: "ui-monospace, monospace" },
          ".cm-ref-ok": { color: "var(--accent)", cursor: "pointer", textDecoration: "underline" },
          ".cm-ref-missing": { color: "var(--warn)", textDecorationLine: "underline", textDecorationStyle: "dotted" },
        }),
      ],
    }),
  });

  rescan(view);
  return view;
}
