<script>import Shell from '../../Shell.svelte'; let {data,children}=$props();</script>
<Shell {data}>{@render children()}</Shell>
