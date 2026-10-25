<script>let {form}=$props();</script>
<svelte:head><title>Sign in · Testbench</title></svelte:head>
<main class="login"><div class="login-brand"><img class="login-logo" src="/hyperswitch-logo.svg" alt="JUSPAY Hyperswitch"/></div><div class="login-heading"><p class="eyebrow">Performance & reliability console</p></div>
  <form method="POST"><label>Username<input name="username" autocomplete="username" required /></label>
    <label>Password<input name="password" type="password" autocomplete="current-password" required /></label>
    {#if form?.error}<p role="alert" class="invalid">{form.error}</p>{/if}<button>Sign in</button>
  </form>
</main>
<style>.login{width:min(420px,calc(100% - 32px));margin:12vh auto 40px;padding:32px;background:var(--surface);border:1px solid var(--border);border-radius:14px;box-shadow:0 10px 40px #18263b06}.login-brand{margin-bottom:10px}.login-logo{display:block;width:205px;max-width:100%;height:auto}.login-heading{margin-bottom:28px}.login label{margin:18px 0}.login button{width:100%;margin-top:8px}.login .invalid{font-size:12px}.login .eyebrow{font-size:10px;margin:0}@media(max-width:480px){.login{padding:24px;margin-top:8vh}}</style>
