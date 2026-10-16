<script>import RunScreen from '../../../RunScreen.svelte'; let {data}=$props();</script>
{#key data.run?.id || data.profileId}<RunScreen {data} tab="performance"/>{/key}
