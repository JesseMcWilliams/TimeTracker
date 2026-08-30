<script lang="ts">
  let {
    onGoToUser,
    onGoToTrash,
    onGoToAppearance,
  }: { onGoToUser: () => void; onGoToTrash: () => void; onGoToAppearance: () => void } = $props()
</script>

<section class="panel">
  <h2>Admin</h2>

  <ul class="menu">
    <li>
      <button class="menu-row" onclick={onGoToUser}>
        <span class="title">User</span>
        <span class="desc">Profile, output folder/format, database size, backup/restore, purge all</span>
      </button>
    </li>
    <li>
      <button class="menu-row" onclick={onGoToTrash}>
        <span class="title">Trash</span>
        <span class="desc">Deleted entries and archived clients/contracts/categories</span>
      </button>
    </li>
    <li>
      <button class="menu-row" onclick={onGoToAppearance}>
        <span class="title">Appearance</span>
        <span class="desc">Window position &amp; size, default start page, colors</span>
      </button>
    </li>
  </ul>
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  .menu {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    max-width: 32rem;
  }
  .menu-row {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.2rem;
    width: 100%;
    background: none;
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    padding: 0.75rem 1rem;
    margin-bottom: 0.6rem;
    cursor: pointer;
    text-align: left;
  }
  .menu-row:hover {
    background: rgba(128, 128, 128, 0.1);
  }
  .title {
    font-size: 1.05rem;
    font-weight: 600;
  }
  .desc {
    font-size: 0.85rem;
    color: #666;
  }
</style>
