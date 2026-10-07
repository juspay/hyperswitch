<script>let {form}=$props();</script>
<svelte:head><title>Sign in / Hyperswitch load testing</title></svelte:head>
<main class="login"><img class="login-logo" src="/hyperswitch-logo.svg" alt="Hyperswitch"/><h1>Load testing</h1>
  <form method="POST"><label>Username<input name="username" autocomplete="username" required /></label>
    <label>Password<input name="password" type="password" autocomplete="current-password" required /></label>
    {#if form?.error}<p role="alert" class="invalid">{form.error}</p>{/if}<button>Sign in</button>
  </form>
</main>
<style>.login{max-width:400px;margin:100px auto;padding:24px}.login-logo{width:230px;height:auto;margin-bottom:14px}.login h1{font-size:18px;margin-bottom:30px}.login label{margin:18px 0}.login button{width:100%;margin-top:12px}</style>
