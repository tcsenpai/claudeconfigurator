<script lang="ts">
  import DocEditor from "../lib/DocEditor.svelte";
  // settings.json has no frontmatter -> DocEditor treats it as raw body,
  // JSON-highlighted, validated on save.
</script>

<div class="wrap">
  <DocEditor path="settings.json" lang="json" validateJson />
</div>

<style>
  .wrap { height: 100%; }
</style>
