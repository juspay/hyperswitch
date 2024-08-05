// @ts-check

/**
 * UTIL FUNCTIONS
 */

/**
 * Ref - https://github.com/onury/invert-color/blob/master/lib/cjs/invert.js
 */
function padz(str, len) {
  if (len === void 0) {
    len = 2;
  }
  return (new Array(len).join("0") + str).slice(-len);
}
function hexToRgbArray(hex) {
  if (hex.slice(0, 1) === "#") hex = hex.slice(1);
  var RE_HEX = /^(?:[0-9a-f]{3}){1,2}$/i;
  if (!RE_HEX.test(hex)) throw new Error('Invalid HEX color: "' + hex + '"');
  if (hex.length === 3) {
    hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
  }
  return [
    parseInt(hex.slice(0, 2), 16),
    parseInt(hex.slice(2, 4), 16),
    parseInt(hex.slice(4, 6), 16),
  ];
}
function toRgbArray(c) {
  if (!c) throw new Error("Invalid color value");
  if (Array.isArray(c)) return c;
  return typeof c === "string" ? hexToRgbArray(c) : [c.r, c.g, c.b];
}
function getLuminance(c) {
  var i, x;
  var a = [];
  for (i = 0; i < c.length; i++) {
    x = c[i] / 255;
    a[i] = x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
  }
  return 0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2];
}
function invertToBW(color, bw, asArr) {
  var DEFAULT_BW = {
    black: "#090302",
    white: "#FFFFFC",
    threshold: Math.sqrt(1.05 * 0.05) - 0.05,
  };
  var options = bw === true ? DEFAULT_BW : Object.assign({}, DEFAULT_BW, bw);
  return getLuminance(color) > options.threshold
    ? asArr
      ? hexToRgbArray(options.black)
      : options.black
    : asArr
    ? hexToRgbArray(options.white)
    : options.white;
}
function invert(color, bw) {
  if (bw === void 0) {
    bw = false;
  }
  color = toRgbArray(color);
  if (bw) return invertToBW(color, bw);
  return (
    "#" +
    color
      .map(function (c) {
        return padz((255 - c).toString(16));
      })
      .join("")
  );
}

const locales = {
  en: {
    expiresOn: "Link expires on: ",
    refId: "Ref Id: ",
    requestedBy: "Requested by ",
    payNow: "Pay now",
    yourCart: "Your Cart",
    quantity: "Quantity",
    showLess: "Show Less",
    showMore: "Show More",
    miscellaneousCharges: "Miscellaneous charges",
    miscellaneousChargesDetail: "(includes taxes, shipping, discounts, offers etc.)",
    paymentTakingLonger: "Sorry! Your payment is taking longer than expected. Please check back again in sometime.",
    paymentLinkExpired: "Payment Link Expired",
    paymentReceived: "We have successfully received your payment",
    paymentLinkExpiredMessage: "Sorry, this payment link has expired. Please use below reference for further investigation.",
    paidSuccessfully: "Paid successfully",
    paymentPending: "Payment Pending",
    paymentFailed: "Payment Failed!",
    paymentCancelled: "Payment Cancelled",
    paymentUnderReview: "Payment under review",
    paymentSuccess: "Payment Success",
    partialPaymentCaptured: "Partial payment was captured.",
    somethingWentWrong: "Something went wrong",
    redirecting: "Redirecting ...",
    redirectingIn: "Redirecting in ",
    seconds: " seconds ...",
    errorCode: "Error code",
    errorMessage: "Error message"
  },
  de: {
    expiresOn: "Link läuft ab am: ",
    refId: "Referenz-ID: ",
    requestedBy: "Angefordert von ",
    payNow: "Jetzt bezahlen",
    yourCart: "Ihr Warenkorb",
    quantity: "Menge",
    showLess: "Weniger anzeigen",
    showMore: "Mehr anzeigen",
    miscellaneousCharges: "Sonstige Gebühren",
    miscellaneousChargesDetail: "(einschließlich Steuern, Versand, Rabatte, Angebote usw.)",
    paymentTakingLonger: "Entschuldigung! Ihre Zahlung dauert länger als erwartet. Bitte prüfen Sie später erneut.",
    paymentLinkExpired: "Zahlungslink abgelaufen",
    paymentReceived: "Wir haben Ihre Zahlung erfolgreich erhalten",
    paymentLinkExpiredMessage: "Entschuldigung, dieser Zahlungslink ist abgelaufen. Bitte verwenden Sie die folgende Referenz für weitere Untersuchungen.",
    paidSuccessfully: "Erfolgreich bezahlt",
    paymentPending: "Zahlung ausstehend",
    paymentFailed: "Zahlung fehlgeschlagen!",
    paymentCancelled: "Zahlung storniert",
    paymentUnderReview: "Zahlung wird überprüft",
    paymentSuccess: "Zahlung erfolgreich",
    partialPaymentCaptured: "Teilzahlung wurde erfasst.",
    somethingWentWrong: "Etwas ist schiefgelaufen",
    redirecting: "Weiterleiten ...",
    redirectingIn: "Weiterleiten in ",
    seconds: " Sekunden ...",
    errorCode: "Fehlercode",
    errorMessage: "Fehlermeldung"
  },
  pt: {
    expiresOn: "Link expira em: ",
    refId: "ID de referência: ",
    requestedBy: "Solicitado por ",
    payNow: "Pagar agora",
    yourCart: "Seu Carrinho",
    quantity: "Quantidade",
    showLess: "Mostrar menos",
    showMore: "Mostrar mais",
    miscellaneousCharges: "Encargos diversos",
    miscellaneousChargesDetail: "(inclui impostos, frete, descontos, ofertas, etc.)",
    paymentTakingLonger: "Desculpe! Seu pagamento está demorando mais do que o esperado. Por favor, verifique novamente em algum tempo.",
    paymentLinkExpired: "Link de Pagamento Expirado",
    paymentReceived: "Recebemos seu pagamento com sucesso",
    paymentLinkExpiredMessage: "Desculpe, este link de pagamento expirou. Por favor, use a referência abaixo para investigação adicional.",
    paidSuccessfully: "Pago com sucesso",
    paymentPending: "Pagamento Pendente",
    paymentFailed: "Pagamento Falhou!",
    paymentCancelled: "Pagamento Cancelado",
    paymentUnderReview: "Pagamento em análise",
    paymentSuccess: "Sucesso no pagamento",
    partialPaymentCaptured: "Pagamento parcial capturado.",
    somethingWentWrong: "Algo deu errado",
    redirecting: "Redirecionando ...",
    redirectingIn: "Redirecionando em ",
    seconds: " segundos ...",
    errorCode: "Código de erro",
    errorMessage: "Mensagem de erro"
  },
  it: {
    expiresOn: "Link scade il: ",
    refId: "ID di riferimento: ",
    requestedBy: "Richiesto da ",
    payNow: "Paga ora",
    yourCart: "Il tuo carrello",
    quantity: "Quantità",
    showLess: "Mostra meno",
    showMore: "Mostra di più",
    miscellaneousCharges: "Spese varie",
    miscellaneousChargesDetail: "(inclusi tasse, spedizione, sconti, offerte, ecc.)",
    paymentTakingLonger: "Spiacenti! Il tuo pagamento sta impiegando più tempo del previsto. Controlla di nuovo tra un po'.",
    paymentLinkExpired: "Link di pagamento scaduto",
    paymentReceived: "Abbiamo ricevuto il tuo pagamento con successo",
    paymentLinkExpiredMessage: "Spiacenti, questo link di pagamento è scaduto. Utilizza il riferimento sottostante per ulteriori indagini.",
    paidSuccessfully: "Pagato con successo",
    paymentPending: "Pagamento in sospeso",
    paymentFailed: "Pagamento fallito!",
    paymentCancelled: "Pagamento annullato",
    paymentUnderReview: "Pagamento in revisione",
    paymentSuccess: "Pagamento riuscito",
    partialPaymentCaptured: "Pagamento parziale catturato.",
    somethingWentWrong: "Qualcosa è andato storto",
    redirecting: "Reindirizzando ...",
    redirectingIn: "Reindirizzando in ",
    seconds: " secondi ...",
    errorCode: "Codice di errore",
    errorMessage: "Messaggio di errore"
  },
  zh: {
    expiresOn: "链接到期日期：",
    refId: "参考编号：",
    requestedBy: "请求者：",
    payNow: "立即支付",
    yourCart: "你的购物车",
    quantity: "数量",
    showLess: "显示更少",
    showMore: "显示更多",
    miscellaneousCharges: "其他费用",
    miscellaneousChargesDetail: "(包括税费、运费、折扣、优惠等)",
    paymentTakingLonger: "对不起！您的付款花费的时间比预期的要长。请稍后再检查。",
    paymentLinkExpired: "支付链接已过期",
    paymentReceived: "我们已经成功收到您的付款",
    paymentLinkExpiredMessage: "对不起，这个支付链接已经过期。请使用下面的参考信息进行进一步调查。",
    paidSuccessfully: "支付成功",
    paymentPending: "付款待处理",
    paymentFailed: "支付失败！",
    paymentCancelled: "支付已取消",
    paymentUnderReview: "支付正在审查",
    paymentSuccess: "支付成功",
    partialPaymentCaptured: "部分付款已捕获。",
    somethingWentWrong: "出了点问题",
    redirecting: "正在重定向 ...",
    redirectingIn: "在 ",
    seconds: " 秒内重定向 ...",
    errorCode: "错误代码",
    errorMessage: "错误信息"
  },
  es: {
    expiresOn: "El enlace expira el: ",
    refId: "ID de referencia: ",
    requestedBy: "Solicitado por ",
    payNow: "Pagar ahora",
    yourCart: "Tu carrito",
    quantity: "Cantidad",
    showLess: "Mostrar menos",
    showMore: "Mostrar más",
    miscellaneousCharges: "Cargos varios",
    miscellaneousChargesDetail: "(incluye impuestos, envío, descuentos, ofertas, etc.)",
    paymentTakingLonger: "¡Lo siento! Tu pago está tardando más de lo esperado. Por favor, vuelve a verificarlo más tarde.",
    paymentLinkExpired: "Enlace de pago expirado",
    paymentReceived: "Hemos recibido tu pago con éxito",
    paymentLinkExpiredMessage: "Lo siento, este enlace de pago ha expirado. Por favor, usa la referencia a continuación para una investigación adicional.",
    paidSuccessfully: "Pagado exitosamente",
    paymentPending: "Pago Pendiente",
    paymentFailed: "¡Pago Fallido!",
    paymentCancelled: "Pago Cancelado",
    paymentUnderReview: "Pago en revisión",
    paymentSuccess: "Éxito en el pago",
    partialPaymentCaptured: "Pago parcial capturado.",
    somethingWentWrong: "Algo salió mal",
    redirecting: "Redirigiendo ...",
    redirectingIn: "Redirigiendo en ",
    seconds: " segundos ...",
    errorCode: "Código de error",
    errorMessage: "Mensaje de error"
  }
};

/**
 * UTIL FUNCTIONS END HERE
 */

// @ts-ignore
{{ payment_details_js_script }}

// @ts-ignore
window.state = {
  prevHeight: window.innerHeight,
  prevWidth: window.innerWidth,
  isMobileView: window.innerWidth <= 1400,
};


function getTranslations() {
  var paymentDetails = window.__PAYMENT_DETAILS;
  console.log(paymentDetails);
  var locale = paymentDetails.locale || 'en'; // defaults if locale is not present in payment details.
  return locales[locale] || locales['en']; // defaults if locale is not implemented in locales.
}

const translations = getTranslations();
/**
 * Trigger - init function invoked once the script tag is loaded
 * Use
 *  - Update document's title
 *  - Update document's icon
 *  - Render and populate document with payment details and cart
 *  - Initialize event listeners for updating UI on screen size changes
 *  - Initialize SDK
 **/
function boot() {
  // @ts-ignore
  var paymentDetails = window.__PAYMENT_DETAILS;

  // Attach document icon
  if (paymentDetails.merchant_logo) {
    var link = document.createElement("link");
    link.rel = "icon";
    link.href = paymentDetails.merchant_logo;
    link.type = "image/x-icon";
    document.head.appendChild(link);
  }

  // Render status details
  renderStatusDetails(paymentDetails);

  // Add event listeners
  initializeEventListeners(paymentDetails);
}

/**
 * Trigger - on boot
 * Uses
 *    - Render status details
 *      - Header - (amount, merchant name, merchant logo)
 *      - Body - status with image
 *      - Footer - payment details (id | error code and msg, if any)
 * @param {PaymentDetails} paymentDetails
 **/
function renderStatusDetails(paymentDetails) {
  var status = paymentDetails.status;

  var statusDetails = {
    imageSource: "",
    message: "",
    status: status,
    amountText: "",
    items: [],
  };

  // Payment details
  var paymentId = createItem(translations.refId, paymentDetails.payment_id);
  // @ts-ignore
  statusDetails.items.push(paymentId);

  // Status specific information
  switch (status) {
    case "expired":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = translations.paymentLinkExpired;
      statusDetails.message = translations.paymentLinkExpiredMessage;
      break;

    case "succeeded":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/success.png";
      statusDetails.message = translations.paymentReceived;
      statusDetails.status = translations.paidSuccessfully;
      statusDetails.amountText = new Date(
        paymentDetails.created
      ).toTimeString();
      break;

    case "processing":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/pending.png";
      statusDetails.message = translations.paymentTakingLonger;
      statusDetails.status = translations.paymentPending;
      break;

    case "failed":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = translations.paymentFailed;
      var errorCodeNode = createItem(translations.errorCode, paymentDetails.error_code);
      var errorMessageNode = createItem(
        translations.errorMessage,
        paymentDetails.error_message
      );
      // @ts-ignore
      statusDetails.items.push(errorMessageNode, errorCodeNode);
      break;

    case "cancelled":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = translations.paymentCancelled;
      break;

    case "requires_merchant_action":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/pending.png";
      statusDetails.status = translations.paymentUnderReview;
      break;

    case "requires_capture":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/success.png";
      statusDetails.message = translations.paymentReceived;
      statusDetails.status = translations.paymentSuccess;
      break;

    case "partially_captured":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/success.png";
      statusDetails.message = translations.partialPaymentCaptured;
      statusDetails.status = translations.paymentSuccess;
      break;

    default:
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = translations.somethingWentWrong;
      // Error details
      if (typeof paymentDetails.error === "object") {
        var errorCodeNode = createItem(translations.errorCode, paymentDetails.error.code);
        var errorMessageNode = createItem(
          translations.errorMessage,
          paymentDetails.error.message
        );
        // @ts-ignore
        statusDetails.items.push(errorMessageNode, errorCodeNode);
      }
      break;
  }

  // Form header items
  var amountNode = document.createElement("div");
  amountNode.className = "hyper-checkout-status-amount";
  amountNode.innerText = paymentDetails.currency + " " + paymentDetails.amount;
  var merchantLogoNode = document.createElement("img");
  merchantLogoNode.className = "hyper-checkout-status-merchant-logo";
  // @ts-ignore
  merchantLogoNode.src = window.__PAYMENT_DETAILS.merchant_logo;
  merchantLogoNode.alt = "";

  // Form content items
  var statusImageNode = document.createElement("img");
  statusImageNode.className = "hyper-checkout-status-image";
  statusImageNode.src = statusDetails.imageSource;
  var statusTextNode = document.createElement("div");
  statusTextNode.className = "hyper-checkout-status-text";
  statusTextNode.innerText = statusDetails.status;
  var statusMessageNode = document.createElement("div");
  statusMessageNode.className = "hyper-checkout-status-message";
  statusMessageNode.innerText = statusDetails.message;
  var statusDetailsNode = document.createElement("div");
  statusDetailsNode.className = "hyper-checkout-status-details";

  // Append items
  if (statusDetailsNode instanceof HTMLDivElement) {
    statusDetails.items.map(function (item) {
      statusDetailsNode.append(item);
    });
  }
  var statusHeaderNode = document.getElementById(
    "hyper-checkout-status-header"
  );
  if (statusHeaderNode instanceof HTMLDivElement) {
    statusHeaderNode.append(amountNode, merchantLogoNode);
  }
  var statusContentNode = document.getElementById(
    "hyper-checkout-status-content"
  );
  if (statusContentNode instanceof HTMLDivElement) {
    statusContentNode.append(statusImageNode, statusTextNode);
    if (statusMessageNode instanceof HTMLDivElement) {
      statusContentNode.append(statusMessageNode);
    }
    statusContentNode.append(statusDetailsNode);
  }

  if (paymentDetails.redirect === true) {
    // Form redirect text
    var statusRedirectTextNode = document.getElementById(
      "hyper-checkout-status-redirect-message"
    );
    if (
      statusRedirectTextNode instanceof HTMLDivElement &&
      typeof paymentDetails.return_url === "string"
    ) {
      var timeout = 5,
        j = 0;
      for (var i = 0; i <= timeout; i++) {
        setTimeout(function () {
          var secondsLeft = timeout - j++;
          var innerText =
            secondsLeft === 0
              ? translations.redirecting
              : translations.redirectingIn + secondsLeft + " "+translations.seconds;
          // @ts-ignore
          statusRedirectTextNode.innerText = innerText;
          if (secondsLeft === 0) {
            // Form query params
            var queryParams = {
              payment_id: paymentDetails.payment_id,
              status: paymentDetails.status,
            };
            var url = new URL(paymentDetails.return_url);
            var params = new URLSearchParams(url.search);
            // Attach query params to return_url
            for (var key in queryParams) {
              if (queryParams.hasOwnProperty(key)) {
                params.set(key, queryParams[key]);
              }
            }
            url.search = params.toString();
            setTimeout(function () {
              // Finally redirect
              window.location.href = url.toString();
            }, 1000);
          }
        }, i * 1000);
      }
    }
  }
}

/**
 * Use - create an item which is a key-value pair of some information related to a payment
 * @param {String} heading
 * @param {String} value
 **/
function createItem(heading, value) {
  var itemNode = document.createElement("div");
  itemNode.className = "hyper-checkout-status-item";
  var headerNode = document.createElement("div");
  headerNode.className = "hyper-checkout-item-header";
  headerNode.innerText = heading;
  var valueNode = document.createElement("div");
  valueNode.className = "hyper-checkout-item-value";
  valueNode.innerText = value;
  itemNode.append(headerNode);
  itemNode.append(valueNode);
  return itemNode;
}

/**
 * Use - add event listeners for changing UI on screen resize
 * @param {PaymentDetails} paymentDetails
 */
function initializeEventListeners(paymentDetails) {
  var primaryColor = paymentDetails.theme;
  var contrastBWColor = invert(primaryColor, true);
  var statusRedirectTextNode = document.getElementById(
    "hyper-checkout-status-redirect-message"
  );

  if (statusRedirectTextNode instanceof HTMLDivElement) {
    statusRedirectTextNode.style.color = contrastBWColor;
  }
  };

