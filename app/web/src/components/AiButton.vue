<template>
  <div v-if="assetStore.selectedVariantId" class="flex flex-row gap-xs items-center">
    <template v-if="open">
      <VormInput
        ref="command"
        v-model="awsCommand.command"
        label="Command"
        type="text"
        :disabled="generateRequestStatus.isPending"
        @enterPressed="generateAwsAssetSchema"
      />
      <VormInput
        ref="subcommand"
        v-model="awsCommand.subcommand"
        label="Subcommand"
        type="text"
        :disabled="generateRequestStatus.isPending"
        @enterPressed="generateAwsAssetSchema"
      />
      <VButton
        :loading="generateRequestStatus.isPending"
        loadingIcon="sparkles"
        loadingText="Generating ..."
        :requestStatus="generateRequestStatus"
        icon="sparkles" size="lg" tooltip="Generate Schema From AWS Command" tooltipPlacement="bottom" @click="generateAwsAssetSchema"
        label="Generate"
      />
      <IconButton icon="x" iconTone="destructive" size="lg" tooltip="Cancel" tooltipPlacement="bottom" @click="setOpen(false)" />
    </template>
    <IconButton v-else icon="sparkles" size="lg" tooltip="Generate AWS Asset Schema" @click="setOpen(true)" />
  </div>
</template>

<script lang="ts" setup>
import { ref, reactive } from "vue";
import { VormInput, IconButton, VButton } from "@si/vue-lib/design-system";
import { useAssetStore } from "@/store/asset.store";

const assetStore = useAssetStore();

/** Whether the form is open or not */
const open = ref(false);
const setOpen = async (value: boolean) => {
  open.value = value;
}

const awsCommand = reactive({ command: "sqs", subcommand: "create-queue" });

const generateRequestStatus = assetStore.getRequestStatus(
  "GENERATE_AWS_ASSET_SCHEMA",
  assetStore.selectedVariantId,
);

const generateAwsAssetSchema = async () => {
  if (!generateRequestStatus.value.isPending && assetStore.selectedVariantId) {
    console.log("Generating AWS Asset Schema for", assetStore.selectedVariantId);
    await assetStore.GENERATE_AWS_ASSET_SCHEMA(assetStore.selectedVariantId, awsCommand.command, awsCommand.subcommand);
  }
}

</script>
