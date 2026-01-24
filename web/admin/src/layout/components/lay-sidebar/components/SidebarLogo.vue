<script setup lang="ts">
import { getTopMenu } from "@/router/utils";
import { useNav } from "@/layout/hooks/useNav";

defineProps({
  collapse: Boolean
});

const { title, getLogo } = useNav();
</script>

<template>
  <div class="sidebar-logo-container" :class="{ collapses: collapse }">
    <transition name="sidebarLogoFade">
      <router-link
        v-if="collapse"
        key="collapse"
        :title="title"
        class="sidebar-logo-link"
        :to="getTopMenu()?.path ?? '/'"
      >
        <img :src="getLogo()" alt="logo" class="logo-img" />
        <span class="sidebar-title">{{ title }}</span>
      </router-link>
      <router-link
        v-else
        key="expand"
        :title="title"
        class="sidebar-logo-link"
        :to="getTopMenu()?.path ?? '/'"
      >
        <img :src="getLogo()" alt="logo" class="logo-img" />
        <span class="sidebar-title">{{ title }}</span>
      </router-link>
    </transition>
  </div>
</template>

<style lang="scss" scoped>
.sidebar-logo-container {
  position: relative;
  width: 100%;
  height: 56px;
  overflow: hidden;
  background: linear-gradient(135deg,
    var(--pure-theme-sidebar-logo) 0%,
    color-mix(in srgb, var(--pure-theme-sidebar-logo) 85%, #000) 100%);
  border-bottom: 1px solid var(--color-cream-200);
  display: flex;
  align-items: center;
  transition: all var(--transition-base);

  html.dark & {
    background: linear-gradient(135deg,
      rgba(26, 26, 26, 0.95) 0%,
      rgba(42, 42, 42, 0.95) 100%);
    border-bottom-color: var(--color-cream-300);
  }
}

.sidebar-logo-link {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  width: 100%;
  height: 100%;
  padding: 0 var(--space-5);
  text-decoration: none;
  transition: all var(--transition-fast);

  &:hover {
    background-color: rgba(0, 0, 0, 0.02);

    html.dark & {
      background-color: rgba(255, 255, 255, 0.03);
    }
  }

  .logo-img {
    display: inline-block;
    height: 36px;
    width: auto;
    object-fit: contain;
    transition: all var(--transition-base);

    &:hover {
      transform: scale(1.05);
    }
  }

  .sidebar-title {
    display: inline-block;
    height: 36px;
    margin-left: var(--space-3);
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-display);
    font-size: 1.25rem;
    font-weight: 600;
    line-height: 36px;
    color: var(--pure-theme-sub-menu-active-text);
    white-space: nowrap;
    letter-spacing: -0.01em;
    transition: all var(--transition-fast);
  }
}

/* Fade transition */
.sidebarLogoFade-enter-active,
.sidebarLogoFade-leave-active {
  transition: opacity var(--transition-base);
}

.sidebarLogoFade-enter-from,
.sidebarLogoFade-leave-to {
  opacity: 0;
}

/* Collapsed state */
.collapses {
  .sidebar-logo-link {
    justify-content: center;
    padding: 0;

    .sidebar-title {
      opacity: 0;
      width: 0;
      margin: 0;
    }

    .logo-img {
      margin: 0;
    }
  }
}

/* Logo icon animation on hover */
:not(.collapses) .sidebar-logo-link:hover .logo-img {
  animation: pulse 1s cubic-bezier(0.4, 0, 0.6, 1);
}

@keyframes pulse {
  0%, 100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.08);
  }
}
</style>
