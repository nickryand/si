<template>
  <Transition
    enterActiveClass="duration-200 ease-out"
    enterFromClass="translate-x-[-230px]"
    leaveActiveClass="duration-200 ease-in"
    leaveToClass="translate-x-[-230px]"
  >
    <div
      v-if="open"
      class="absolute w-[230px] h-full left-[0px] bg-white dark:bg-neutral-800 z-100"
    >
      <div
        class="flex flex-row items-center gap-xs m-xs mt-0 font-bold border-b dark:border-neutral-500 py-2xs"
      >
        <Icon
          class="cursor-pointer"
          name="x-circle"
          size="sm"
          @click="() => emit('closed')"
        />
        <div
          class="uppercase text-md leading-6 text-neutral-500 dark:text-neutral-400"
        >
          Views
        </div>
        <PillCounter
          :count="viewCount"
          hideIfZero
          :paddingX="viewCount < 10 ? 'xs' : '2xs'"
        />
        <slot />
      </div>
    </div>
  </Transition>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { Icon, PillCounter } from "@si/vue-lib/design-system";
import { useViewsStore } from "@/store/views.store";

const viewStore = useViewsStore();

defineProps({
  open: { type: Boolean },
});

const emit = defineEmits<{
  (e: "closed"): void;
}>();

const viewCount = computed(() => viewStore.viewList.length);
</script>
