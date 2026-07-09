<script lang="ts">
  import DocEditor from "../lib/DocEditor.svelte";
  import { follow } from "../lib/nav.svelte";
  // The entrypoint file gets its own tab.
</script>

<DocEditor path="CLAUDE.md" onFollow={follow} />
