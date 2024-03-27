<!-- You can run hyperswitch, hyperswitch-web, hyperswitch-react-demo-app and hypersiwtch-control-center all by running this in terminal:
1. git clone https://github.com/juspay/hyperswitch
2. git clone https://github.com/adityak-21/hyperswitch-web.git
3. cd hyperswitch
4. docker compose up -d

What I did:
1. Added a dockerfile in hyperswitch-web which generates an image
2. Changed docker-compose file in hyperswitch for running all the images in the same network -->
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Run Hyperswitch and Components</title>
</head>
<body>

<h1>Run Hyperswitch and its Components</h1>

<ol>
  <li>Clone the Hyperswitch repository:</li>
  <pre><code>git clone https://github.com/juspay/hyperswitch</code></pre>

  <li>Clone the Hyperswitch Web repository:</li>
  <pre><code>git clone https://github.com/adityak-21/hyperswitch-web.git</code></pre>

  <li>Navigate to the Hyperswitch directory:</li>
  <pre><code>cd hyperswitch</code></pre>

  <li>Run Hyperswitch and its components:</li>
  <pre><code>docker-compose up -d</code></pre>
</ol>

<p>By following these steps, you should have Hyperswitch, Hyperswitch Web, Hyperswitch React Demo App, and Hyperswitch Control Center all up and running.</p>

<h1>What I changed:</h1>

<ol>
    <li>Added a Dockerfile in the <code>hyperswitch-web</code> directory to generate an image.</li>
    <li>Modified the Docker Compose file in the Hyperswitch directory to run all the images in the same network.</li>
</ol>

</body>
</html>
