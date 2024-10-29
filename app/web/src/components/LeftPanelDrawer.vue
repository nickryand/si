<template>
  <Transition
    enterActiveClass="duration-200 ease-out"
    enterFromClass="translate-x-[-230px]"
    leaveActiveClass="duration-200 ease-in"
    leaveToClass="translate-x-[-230px]"
  >
    <div
      v-if="open"
      class="absolute w-[230px] h-full left-[0px] bg-white dark:bg-neutral-800 z-100 border-r-[3px] shadow-[4px_0_6px_3px_rgba(0,0,0,0.33)] border-neutral-300 border-color: dark:border-neutral-600"
    >
      <div
        class="flex flex-row items-center gap-xs pl-xs font-bold border-b dark:border-neutral-500 py-2xs"
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
      </div>

      <SiSearch
        ref="searchRef"
        placeholder="search views"
        @search="onSearchUpdated"
      />

      <div>
        <ViewCard v-for="view in filteredViews" :key="view.id" :view="view" />
      </div>
    </div>
  </Transition>
</template>

<script lang="ts" setup>
import { computed, ref } from "vue";
import { Icon, PillCounter, SiSearch } from "@si/vue-lib/design-system";
import { useViewsStore } from "@/store/views.store";
import ViewCard from "./ViewCard.vue";

const viewStore = useViewsStore();

defineProps({
  open: { type: Boolean },
});

const emit = defineEmits<{
  (e: "closed"): void;
}>();

const viewCount = computed(() => viewStore.viewList.length);

const searchRef = ref<InstanceType<typeof SiSearch>>();
const searchTerm = ref("");

const onSearchUpdated = (q: string) => {
  searchTerm.value = q;
};

const filteredViews = computed(() => {
  if (!searchTerm.value) return viewStore.viewList;
  return viewStore.viewList.filter((v) =>
    v.name.toLowerCase().includes(searchTerm.value.toLowerCase()),
  );
});
</script>
