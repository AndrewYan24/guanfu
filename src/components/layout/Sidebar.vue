<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectStore } from '@/stores/projectStore';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';

defineProps<{
  expanded: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
}>();

const router = useRouter();
const route = useRoute();
const { t } = useI18n();
const projectStore = useProjectStore();
const paperStore = usePaperStore();
const graphStore = useGraphStore();

const navItems = [
  { path: '/library', labelKey: 'sidebar.library', icon: 'library' },
  { path: '/graph', labelKey: 'sidebar.graph', icon: 'graph' },
  { path: '/insights', labelKey: 'sidebar.insights', icon: 'insights' },
  { path: '/chat', labelKey: 'sidebar.chat', icon: 'chat' },
];

function navigate(path: string) {
  if (route.path === path) return;
  router.push(path).catch(() => {});
}

function handleNewProject() {
  projectStore.showCreateDialog = true;
}

async function handleOpenProject() {
  const dir = await open({
    directory: true,
    multiple: false,
    title: t('library.openProject'),
  });
  if (!dir) return;
  try {
    const project = await projectStore.openProject(dir);
    if (project) {
      paperStore.loadFromProject(project.papers);
      graphStore.loadFromProject(project.relations, project.graphLayout);
      navigate('/library');
    }
  } catch {
    // error handled by store
  }
}
</script>

<template>
  <nav class="sidebar" :class="{ expanded }">
    <div class="sidebar-top">
      <button class="toggle-btn" @click="emit('toggle')" :title="expanded ? t('common.close') : t('common.appName')">
        <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
          <rect x="3" y="4" width="14" height="1.5" rx="0.5" fill="currentColor" />
          <rect x="3" y="9" width="14" height="1.5" rx="0.5" fill="currentColor" />
          <rect x="3" y="14" width="14" height="1.5" rx="0.5" fill="currentColor" />
        </svg>
      </button>
      <span v-if="expanded" class="app-title">{{ t('common.appName') }}</span>
    </div>

    <div class="nav-items">
      <button
        v-for="item in navItems"
        :key="item.path"
        class="nav-item"
        :class="{ active: route.path === item.path }"
        @click="navigate(item.path)"
        :title="t(item.labelKey)"
      >
        <span class="nav-icon">
          <!-- Library icon -->
          <svg v-if="item.icon === 'library'" width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M2 3h5v12H2V3zm6.5 0H14a2 2 0 012 2v8a2 2 0 01-2 2H8.5V3z" stroke="currentColor" stroke-width="1.2" fill="none"/>
          </svg>
          <!-- Graph icon -->
          <svg v-else-if="item.icon === 'graph'" width="18" height="18" viewBox="0 0 18 18" fill="none">
            <circle cx="5" cy="5" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <circle cx="13" cy="5" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <circle cx="9" cy="13" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <line x1="6.8" y1="6" x2="11.2" y2="6" stroke="currentColor" stroke-width="1"/>
            <line x1="5.5" y1="6.8" x2="8.2" y2="11.2" stroke="currentColor" stroke-width="1"/>
            <line x1="12.5" y1="6.8" x2="9.8" y2="11.2" stroke="currentColor" stroke-width="1"/>
          </svg>
          <!-- Insights icon -->
          <svg v-else-if="item.icon === 'insights'" width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M9 1C5.13 1 2 4.13 2 8c0 2.38 1.19 4.47 3 5.74V15a1 1 0 001 1h6a1 1 0 001-1v-1.26c1.81-1.27 3-3.36 3-5.74 0-3.87-3.13-7-7-7z" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <line x1="7" y1="17" x2="11" y2="17" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            <line x1="6.5" y1="12" x2="11.5" y2="12" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
            <line x1="9" y1="5" x2="9" y2="9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            <line x1="7" y1="7" x2="11" y2="7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
          <!-- Chat icon -->
          <svg v-else-if="item.icon === 'chat'" width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M4 3h10a1 1 0 011 1v8a1 1 0 01-1 1H6l-3 3V4a1 1 0 011-1z" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <line x1="6" y1="7" x2="12" y2="7" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
            <line x1="6" y1="10" x2="10" y2="10" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
        </span>
        <span v-if="expanded" class="nav-label">{{ t(item.labelKey) }}</span>
      </button>
    </div>

    <div class="sidebar-bottom">
      <div v-if="expanded && projectStore.hasProject" class="project-info">
        <span class="project-name" :title="projectStore.currentProject?.name">
          {{ projectStore.currentProject?.name }}
        </span>
      </div>

      <button class="nav-item project-action" @click="handleNewProject" :title="t('library.createProject')">
        <span class="nav-icon">
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <rect x="2" y="2" width="14" height="14" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <line x1="9" y1="5" x2="9" y2="13" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            <line x1="5" y1="9" x2="13" y2="9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </span>
        <span v-if="expanded" class="nav-label">{{ t('library.createProject') }}</span>
      </button>

      <button class="nav-item project-action" @click="handleOpenProject" :title="t('library.openProject')">
        <span class="nav-icon">
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M2 6V14a1 1 0 001 1h12a1 1 0 001-1V7a1 1 0 00-1-1h-5L7 4H3a1 1 0 00-1 2z" stroke="currentColor" stroke-width="1.2" fill="none"/>
          </svg>
        </span>
        <span v-if="expanded" class="nav-label">{{ t('library.openProject') }}</span>
      </button>

      <div class="sidebar-divider" />

      <button
        class="nav-item"
        :class="{ active: route.path === '/settings' }"
        @click="navigate('/settings')"
        :title="t('sidebar.settings')"
      >
        <span class="nav-icon">
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M15.2 7.84l-1.54-.26a5.8 5.8 0 00-.6-1.44l.88-1.28-1.42-1.42-1.28.88a5.8 5.8 0 00-1.44-.6L9.54 2.8H8.46l-.26 1.54a5.8 5.8 0 00-1.44.6l-1.28-.88L4.06 5.48l.88 1.28a5.8 5.8 0 00-.6 1.44L2.8 8.46v1.08l1.54.26a5.8 5.8 0 00.6 1.44l-.88 1.28 1.42 1.42 1.28-.88a5.8 5.8 0 001.44.6l.26 1.54h1.08l.26-1.54a5.8 5.8 0 001.44-.6l1.28.88 1.42-1.42-.88-1.28a5.8 5.8 0 00.6-1.44l1.54-.26V7.84z" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <circle cx="9" cy="9" r="2.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
          </svg>
        </span>
        <span v-if="expanded" class="nav-label">{{ t('sidebar.settings') }}</span>
      </button>
    </div>
  </nav>
</template>

<style lang="scss" scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  width: $sidebar-collapsed;
  min-width: $sidebar-collapsed;
  height: 100%;
  background: $color-bg;
  border-right: 1px solid $color-border;
  transition: width $transition-normal, min-width $transition-normal;
  overflow: hidden;

  &.expanded {
    width: $sidebar-expanded;
    min-width: $sidebar-expanded;
  }
}

.sidebar-top {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  padding: $spacing-md;
  height: 48px;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;
}

.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: none;
  color: $color-text-secondary;
  cursor: pointer;
  border-radius: $radius-sm;
  flex-shrink: 0;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }
}

.app-title {
  font-size: 14px;
  font-weight: 600;
  color: $color-text-primary;
  white-space: nowrap;
}

.nav-items {
  flex: 1;
  padding: $spacing-sm 0;
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  width: 100%;
  padding: $spacing-sm $spacing-md;
  border: none;
  background: none;
  color: $color-text-secondary;
  cursor: pointer;
  font-size: 13px;
  text-align: left;
  font-family: $font-family;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }

  &.active {
    color: $color-text-primary;
    background: $color-panel;

    .nav-icon {
      color: $color-text-primary;
    }
  }
}

.nav-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  flex-shrink: 0;
}

.nav-label {
  white-space: nowrap;
}

.sidebar-bottom {
  flex-shrink: 0;
  padding: $spacing-sm 0;
  border-top: 1px solid $color-border;
}

.project-info {
  padding: $spacing-xs $spacing-md;
  margin-bottom: $spacing-xs;
}

.project-name {
  font-size: 11px;
  color: $color-text-disabled;
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-action {
  font-size: 12px;
  color: $color-text-disabled;

  &:hover {
    color: $color-text-primary;
  }
}

.sidebar-divider {
  height: 1px;
  background: $color-border;
  margin: $spacing-xs $spacing-md;
}
</style>
