<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePaperStore } from '@/stores/paperStore';

const { t } = useI18n();
const paperStore = usePaperStore();

const papers = computed(() => paperStore.filteredPapers);

const contextMenuPaperId = ref<string | null>(null);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const showSortMenu = ref(false);
const showTagMenu = ref(false);

// Debounced search
const searchInput = ref(paperStore.searchQuery);
const searchInputRef = ref<HTMLInputElement | null>(null);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchInput, (val) => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    paperStore.searchQuery = val;
  }, 200);
});

// Sync back when store resets externally
watch(() => paperStore.searchQuery, (val) => {
  if (val !== searchInput.value) searchInput.value = val;
});

function focusSearch() {
  searchInputRef.value?.focus();
  searchInputRef.value?.select();
}

function navigatePaper(offset: number) {
  const list = papers.value;
  if (list.length === 0) return;
  const currentIdx = list.findIndex(p => p.id === paperStore.selectedPaperId);
  const nextIdx = currentIdx === -1
    ? (offset > 0 ? 0 : list.length - 1)
    : Math.max(0, Math.min(list.length - 1, currentIdx + offset));
  paperStore.selectPaper(list[nextIdx].id);
}

function handleKeyDown(e: KeyboardEvent) {
  const isMod = e.metaKey || e.ctrlKey;

  // Cmd/Ctrl+F → focus search
  if (isMod && e.key === 'f') {
    e.preventDefault();
    focusSearch();
    return;
  }

  // Don't handle shortcuts when typing in inputs
  const tag = (e.target as HTMLElement).tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') {
    if (e.key === 'Escape') {
      (e.target as HTMLElement).blur();
    }
    return;
  }

  // Arrow keys navigate papers
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    navigatePaper(1);
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    navigatePaper(-1);
  }
}

function handleContextMenu(e: MouseEvent, paperId: string) {
  e.preventDefault();
  contextMenuPaperId.value = paperId;
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
}

function closeContextMenu() {
  contextMenuPaperId.value = null;
}

async function handleDelete() {
  const id = contextMenuPaperId.value;
  closeContextMenu();
  if (!id) return;
  if (!window.confirm(t('library.deleteConfirm'))) return;
  await paperStore.deletePaper(id);
}

function handleCopyTitle() {
  const id = contextMenuPaperId.value;
  closeContextMenu();
  if (!id) return;
  const paper = paperStore.papers.find(p => p.id === id);
  if (paper) {
    navigator.clipboard.writeText(paper.title);
  }
}

async function handleReparse() {
  const id = contextMenuPaperId.value;
  closeContextMenu();
  if (!id) return;
  const projectStore = (await import('@/stores/projectStore')).useProjectStore();
  if (!projectStore.projectPath) return;
  await paperStore.reparsePaper(id);
}

function handleGlobalClick() {
  closeContextMenu();
  showSortMenu.value = false;
  showTagMenu.value = false;
}

function setSort(field: 'title' | 'year' | 'author' | 'added') {
  if (paperStore.sortField === field) {
    paperStore.sortDir = paperStore.sortDir === 'asc' ? 'desc' : 'asc';
  } else {
    paperStore.sortField = field;
    paperStore.sortDir = field === 'added' ? 'desc' : 'asc';
  }
  showSortMenu.value = false;
}

function selectTag(tag: string | null) {
  paperStore.activeTag = tag;
  showTagMenu.value = false;
}

const sortLabel = computed(() => {
  const labels: Record<string, string> = {
    added: t('library.sortByAdded'),
    title: t('library.sortByTitle'),
    year: t('library.sortByYear'),
    author: t('library.sortByAuthor'),
  };
  return labels[paperStore.sortField] || t('library.sortByAdded');
});

onMounted(() => {
  document.addEventListener('click', handleGlobalClick);
  document.addEventListener('keydown', handleKeyDown);
});

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick);
  document.removeEventListener('keydown', handleKeyDown);
  if (searchTimer) clearTimeout(searchTimer);
});
</script>

<template>
  <div class="paper-list">
    <!-- Search / sort / filter bar -->
    <div class="filter-bar">
      <div class="search-wrapper">
        <svg class="search-icon" width="13" height="13" viewBox="0 0 13 13" fill="none">
          <circle cx="5.5" cy="5.5" r="4" stroke="currentColor" stroke-width="1.1"/>
          <line x1="8.5" y1="8.5" x2="11.5" y2="11.5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
        </svg>
        <input
          ref="searchInputRef"
          v-model="searchInput"
          class="search-input"
          :placeholder="t('library.searchPapers')"
        />
        <button
          v-if="searchInput"
          class="search-clear"
          @click="searchInput = ''; paperStore.searchQuery = ''"
        >
          <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
            <line x1="2" y1="2" x2="9" y2="9" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
            <line x1="9" y1="2" x2="2" y2="9" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="filter-actions">
        <div class="dropdown-wrapper">
          <button class="filter-btn" :class="{ active: showSortMenu }" @click.stop="showSortMenu = !showSortMenu; showTagMenu = false">
            <span>{{ sortLabel }}</span>
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M3 4l2 2 2-2" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <div v-if="showSortMenu" class="dropdown-menu">
            <button class="dropdown-item" :class="{ active: paperStore.sortField === 'added' }" @click="setSort('added')">
              {{ t('library.sortByAdded') }}
            </button>
            <button class="dropdown-item" :class="{ active: paperStore.sortField === 'title' }" @click="setSort('title')">
              {{ t('library.sortByTitle') }}
            </button>
            <button class="dropdown-item" :class="{ active: paperStore.sortField === 'year' }" @click="setSort('year')">
              {{ t('library.sortByYear') }}
            </button>
            <button class="dropdown-item" :class="{ active: paperStore.sortField === 'author' }" @click="setSort('author')">
              {{ t('library.sortByAuthor') }}
            </button>
            <div class="dropdown-divider" />
            <button class="dropdown-item" @click="paperStore.sortDir = paperStore.sortDir === 'asc' ? 'desc' : 'asc'; showSortMenu = false">
              {{ paperStore.sortDir === 'asc' ? t('library.sortDesc') : t('library.sortAsc') }}
            </button>
          </div>
        </div>
        <div class="dropdown-wrapper" v-if="paperStore.allTags.length > 0">
          <button class="filter-btn" :class="{ active: !!paperStore.activeTag || showTagMenu }" @click.stop="showTagMenu = !showTagMenu; showSortMenu = false">
            <span>{{ paperStore.activeTag || t('library.filterByTag') }}</span>
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M3 4l2 2 2-2" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <div v-if="showTagMenu" class="dropdown-menu">
            <button class="dropdown-item" :class="{ active: !paperStore.activeTag }" @click="selectTag(null)">
              {{ t('library.allTags') }}
            </button>
            <div class="dropdown-divider" />
            <button
              v-for="tag in paperStore.allTags"
              :key="tag"
              class="dropdown-item"
              :class="{ active: paperStore.activeTag === tag }"
              @click="selectTag(tag)"
            >
              {{ tag }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Result count -->
    <div v-if="paperStore.searchQuery || paperStore.activeTag" class="filter-result-count">
      {{ t('library.paperCount', { count: papers.length }) }}
    </div>

    <!-- Parse progress bar -->
    <div v-if="paperStore.isAutoResolving && paperStore.parseProgress" class="parse-progress-bar">
      <div class="parse-progress-label">
        {{ t('metadata.discovering') }} {{ paperStore.parseProgress.done }}/{{ paperStore.parseProgress.total }}
      </div>
      <div class="parse-progress-track">
        <div
          class="parse-progress-fill"
          :style="{ width: (paperStore.parseProgress.done / paperStore.parseProgress.total * 100) + '%' }"
        />
      </div>
    </div>

    <!-- Parse errors -->
    <div v-if="paperStore.parseErrors.length > 0" class="parse-errors">
      <div class="parse-errors-header">
        <span>{{ paperStore.parseErrors.length }} 篇解析失败</span>
        <button class="parse-errors-dismiss" @click="paperStore.clearParseErrors()">×</button>
      </div>
      <div v-for="err in paperStore.parseErrors" :key="err.paperId" class="parse-error-item">
        <span class="parse-error-title">{{ err.paperTitle }}</span>
        <span class="parse-error-msg">{{ err.error }}</span>
      </div>
    </div>

    <div v-if="papers.length === 0" class="empty-list">
      <p>{{ paperStore.searchQuery || paperStore.activeTag ? t('library.noResults') : t('library.dropPdfs') }}</p>
    </div>
    <div
      v-for="paper in papers"
      :key="paper.id"
      class="paper-item"
      :class="{ selected: paperStore.selectedPaperId === paper.id, parsing: paperStore.isPaperParsing(paper.id) }"
      @click="paperStore.selectPaper(paper.id)"
      @contextmenu="handleContextMenu($event, paper.id)"
    >
      <div class="paper-info">
        <span class="paper-title">
          <span class="paper-title-text">{{ paper.title || t('common.empty') }}</span>
          <span v-if="paperStore.isPaperParsing(paper.id)" class="parse-spinner" />
        </span>
        <span class="paper-meta">
          <template v-if="paperStore.isPaperParsing(paper.id)">
            <span class="parsing-text">{{ t('metadata.discovering') }}</span>
          </template>
          <template v-else-if="paperStore.isPaperQueued(paper.id)">
            <span class="parsing-text">{{ t('metadata.queued') }}</span>
          </template>
          <template v-else>
            {{ paper.authors[0] || '—' }}
            <template v-if="paper.year"> · {{ paper.year }}</template>
          </template>
        </span>
      </div>
      <div class="paper-indicators">
        <span v-if="paper.tags.length" class="indicator tags">
          {{ paper.tags.length }}
        </span>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="contextMenuPaperId"
        class="context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
      >
        <button class="context-menu-item" @click="handleCopyTitle">{{ t('library.copyTitle') }}</button>
        <button class="context-menu-item" @click="handleReparse">{{ t('library.reparse') }}</button>
        <div class="dropdown-divider" />
        <button class="context-menu-item danger" @click="handleDelete">{{ t('library.deletePaper') }}</button>
      </div>
    </Teleport>
  </div>
</template>

<style lang="scss" scoped>
.paper-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.filter-bar {
  padding: $spacing-sm $spacing-md;
  border-bottom: 1px solid $color-border;
  display: flex;
  flex-direction: column;
  gap: $spacing-xs;
  flex-shrink: 0;
}

.search-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: 8px;
  color: $color-text-disabled;
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: 6px 26px 6px 28px;
  border: none;
  border-radius: $radius-sm;
  font-size: 12px;
  font-family: $font-family;
  background: $color-panel;
  color: $color-text-primary;
  transition: background $transition-fast;

  &:focus {
    outline: none;
    background: var(--hover-bg, rgba(0,0,0,0.04));
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.search-clear {
  position: absolute;
  right: 6px;
  border: none;
  background: none;
  color: $color-text-disabled;
  cursor: pointer;
  padding: 2px;
  display: flex;
  align-items: center;
  border-radius: 50%;

  &:hover {
    color: $color-text-primary;
    background: rgba(0,0,0,0.06);
  }
}

.filter-actions {
  display: flex;
  gap: $spacing-xs;
}

.dropdown-wrapper {
  position: relative;
  flex: 1;
}

.filter-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  padding: 4px 6px;
  border: none;
  border-radius: $radius-sm;
  background: transparent;
  color: $color-text-secondary;
  font-size: 11px;
  font-family: $font-family;
  cursor: pointer;
  white-space: nowrap;
  transition: all $transition-fast;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }

  &.active {
    background: $color-panel;
    color: $color-text-primary;
  }

  span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 2px);
  left: 0;
  z-index: 50;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-md;
  box-shadow: $shadow-md;
  padding: 4px 0;
  min-width: 130px;
  max-height: 240px;
  overflow-y: auto;
}

.dropdown-item {
  display: block;
  width: 100%;
  padding: 5px 12px;
  border: none;
  background: none;
  font-size: 12px;
  font-family: $font-family;
  text-align: left;
  cursor: pointer;
  color: $color-text-primary;
  white-space: nowrap;

  &:hover {
    background: $color-panel;
  }

  &.active {
    color: $color-text-primary;
    font-weight: 500;
  }
}

.dropdown-divider {
  height: 1px;
  background: $color-border;
  margin: 4px 0;
}

.filter-result-count {
  padding: 3px $spacing-md;
  font-size: 11px;
  color: $color-text-disabled;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;
}

.parse-progress-bar {
  padding: $spacing-sm $spacing-lg;
  border-bottom: 1px solid $color-border;
}

.parse-progress-label {
  font-size: 11px;
  color: $color-text-secondary;
  margin-bottom: 4px;
}

.parse-progress-track {
  height: 3px;
  background: $color-border;
  border-radius: 2px;
  overflow: hidden;
}

.parse-progress-fill {
  height: 100%;
  background: $color-text-primary;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.parse-errors {
  border-bottom: 1px solid $color-border;
  padding: $spacing-sm $spacing-lg;
  background: var(--hover-bg);
}

.parse-errors-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  color: var(--color-error);
  font-weight: 500;
  margin-bottom: 4px;
}

.parse-errors-dismiss {
  border: none;
  background: none;
  color: $color-text-disabled;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0 2px;
}

.parse-error-item {
  font-size: 11px;
  padding: 2px 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.parse-error-title {
  color: $color-text-primary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.parse-error-msg {
  color: $color-text-disabled;
  font-size: 10px;
}

.empty-list {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: $color-text-disabled;
  font-size: 13px;
}

.paper-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-md $spacing-lg;
  border-bottom: 1px solid $color-border;
  cursor: pointer;
  transition: background $transition-fast;

  &:hover {
    background: $color-panel;
  }

  &.selected {
    background: $color-panel;
    border-left: 2px solid $color-text-primary;
  }
}

.paper-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.paper-title {
  font-size: 13px;
  font-weight: 450;
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.paper-title-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.paper-meta {
  font-size: 12px;
  color: $color-text-secondary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.paper-indicators {
  display: flex;
  gap: $spacing-xs;
  flex-shrink: 0;
}

.indicator {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 2px;
  background: $color-panel;
  color: $color-text-secondary;
  border: 1px solid $color-border;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.parse-spinner {
  flex-shrink: 0;
  width: 14px;
  height: 14px;
  border: 2px solid $color-border;
  border-top-color: $color-text-primary;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.parsing {
  background: var(--hover-bg) !important;
}

.parsing-text {
  color: $color-text-primary;
  font-size: 12px;
}
</style>

<style lang="scss">
.context-menu {
  position: fixed;
  z-index: 200;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  box-shadow: $shadow-md;
  padding: 4px 0;
  min-width: 100px;

  .dropdown-divider {
    height: 1px;
    background: $color-border;
    margin: 4px 0;
  }
}

.context-menu-item {
  display: block;
  width: 100%;
  padding: 6px 14px;
  border: none;
  background: none;
  font-size: 12px;
  font-family: $font-family;
  text-align: left;
  cursor: pointer;
  color: $color-text-primary;

  &:hover {
    background: $color-panel;
  }

  &.danger {
    color: var(--color-error);
  }
}
</style>
