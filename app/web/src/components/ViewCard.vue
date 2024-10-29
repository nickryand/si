<template>
  <div
    :class="
      clsx(
        'flex flex-row items-center text-sm relative p-2xs pl-xs min-w-0 w-full border border-transparent cursor-pointer',
        selected
          ? 'dark:bg-action-900 bg-action-100 border-action-500 dark:border-action-300'
          : 'dark:border-neutral-700',
      )
    "
  >
    <div class="flex flex-col flex-grow min-w-0">
      <TruncateWithTooltip class="w-full">
        <span class="font-bold">
          {{ view.name }}
        </span>
      </TruncateWithTooltip>
    </div>
    <DropdownMenu ref="contextMenuRef" :forceAbove="false" forceAlignRight>
      <DropdownMenuItem :onSelect="() => {}" label="Edit Name" />
      <DropdownMenuItem :onSelect="() => {}" label="Delete View" />
    </DropdownMenu>
    <DetailsPanelMenuIcon
      :selected="contextMenuRef?.isOpen"
      @click="
        (e) => {
          contextMenuRef?.open(e, false);
        }
      "
    />
  </div>
</template>

<script lang="ts" setup>
import { ref } from "vue";

import {
  DropdownMenu,
  DropdownMenuItem,
  TruncateWithTooltip,
} from "@si/vue-lib/design-system";
import clsx from "clsx";
import { ViewDescription } from "@/api/sdf/dal/views";
import { useViewsStore } from "@/store/views.store";
import DetailsPanelMenuIcon from "./DetailsPanelMenuIcon.vue";

const viewStore = useViewsStore();

const props = defineProps<{
  selected?: boolean;
  view: ViewDescription;
}>();

// const confirmRef = ref<InstanceType<typeof ConfirmHoldModal> | null>(null);

const contextMenuRef = ref<InstanceType<typeof DropdownMenu>>();

const emit = defineEmits<{
  (e: "closeDrawer"): void;
}>();
</script>
