<?php
/**
 * 2007-2024 Juspay Technologies
 *
 * NOTICE OF LICENSE
 *
 * This source file is subject to the Academic Free License (AFL 3.0)
 * that is bundled with this package in the file LICENSE.txt.
 * It is also available through the world-wide-web at this URL:
 * http://opensource.org/licenses/afl-3.0.php
 * If you did not receive a copy of the license and are unable to
 * obtain it through the world-wide-web, please send an email
 * to license@prestashop.com so we can send you a copy immediately.
 *
 * DISCLAIMER
 *
 * Do not edit or add to this file if you wish to upgrade PrestaShop to newer
 * versions in the future. If you wish to customize PrestaShop for your
 * needs please refer to http://www.prestashop.com for more information.
 *
 * @author    Juspay Technologies <contact@juspay.in>
 * @copyright 2007-2024 Juspay Technologies
 * @license   http://opensource.org/licenses/afl-3.0.php  Academic Free License (AFL 3.0)
 */

if (!defined('_PS_VERSION_')) {
    exit;
}

require_once __DIR__ . '/vendor/autoload.php';

class Hyperswitch extends PaymentModule
{
    protected $config_form = false;

    public function __construct()
    {
        $this->name = 'hyperswitch';
        $this->tab = 'payments_gateways';
        $this->version = '1.0.0';
        $this->author = 'Juspay Technologies';
        $this->need_instance = 0;

        $this->bootstrap = true;

        parent::__construct();

        $this->displayName = $this->l('Hyperswitch');
        $this->description = $this->l('Accept payments through Hyperswitch payment gateway');

        $this->confirmUninstall = $this->l('Are you sure you want to uninstall?');

        $this->limited_currencies = array('USD', 'EUR', 'GBP', 'INR');
        $this->ps_versions_compliancy = array('min' => '1.7', 'max' => _PS_VERSION_);
    }

    public function install()
    {
        if (extension_loaded('curl') == false) {
            $this->_errors[] = $this->l('You need to enable the cURL extension on your server to install this module');
            return false;
        }

        $iso_code = Country::getIsoById(Configuration::get('PS_COUNTRY_DEFAULT'));

        if (in_array($iso_code, $this->limited_countries) == false) {
            $this->_errors[] = $this->l('This module is not available in your country');
            return false;
        }

        Configuration::updateValue('HYPERSWITCH_LIVE_MODE', false);
        Configuration::updateValue('HYPERSWITCH_API_KEY', '');
        Configuration::updateValue('HYPERSWITCH_PUBLISHABLE_KEY', '');
        Configuration::updateValue('HYPERSWITCH_WEBHOOK_SECRET', '');

        // Install order statuses
        if (!$this->installOrderStatuses()) {
            return false;
        }

        return parent::install() &&
            $this->registerHook('header') &&
            $this->registerHook('backOfficeHeader') &&
            $this->registerHook('paymentOptions') &&
            $this->registerHook('paymentReturn') &&
            $this->registerHook('displayOrderConfirmation') &&
            $this->registerHook('actionFrontControllerSetMedia');
    }

    private function installOrderStatuses()
    {
        $orderStateAwaiting = new OrderState();
        $orderStateAwaiting->color = '#34209E';
        $orderStateAwaiting->module_name = $this->name;
        $orderStateAwaiting->unremovable = true;
        $orderStateAwaiting->hidden = false;
        $orderStateAwaiting->logable = false;
        $orderStateAwaiting->invoice = false;
        $orderStateAwaiting->send_email = false;
        $orderStateAwaiting->paid = false;
        $orderStateAwaiting->pdf_invoice = false;
        $orderStateAwaiting->pdf_delivery = false;
        $orderStateAwaiting->shipped = false;
        $orderStateAwaiting->delivery = false;
        $orderStateAwaiting->deleted = false;

        $languages = Language::getLanguages();
        foreach ($languages as $language) {
            $orderStateAwaiting->name[$language['id_lang']] = 'Awaiting Hyperswitch Payment';
        }

        if (!$orderStateAwaiting->add()) {
            return false;
        }
        Configuration::updateValue('PS_OS_HYPERSWITCH_AWAITING', $orderStateAwaiting->id);

        $orderStateAuthorized = new OrderState();
        $orderStateAuthorized->color = '#4169E1';
        $orderStateAuthorized->module_name = $this->name;
        $orderStateAuthorized->unremovable = true;
        $orderStateAuthorized->hidden = false;
        $orderStateAuthorized->logable = false;
        $orderStateAuthorized->invoice = false;
        $orderStateAuthorized->send_email = false;
        $orderStateAuthorized->paid = false;
        $orderStateAuthorized->pdf_invoice = false;
        $orderStateAuthorized->pdf_delivery = false;
        $orderStateAuthorized->shipped = false;
        $orderStateAuthorized->delivery = false;
        $orderStateAuthorized->deleted = false;

        foreach ($languages as $language) {
            $orderStateAuthorized->name[$language['id_lang']] = 'Hyperswitch Authorized';
        }

        if (!$orderStateAuthorized->add()) {
            return false;
        }
        Configuration::updateValue('PS_OS_HYPERSWITCH_AUTHORIZED', $orderStateAuthorized->id);

        return true;
    }

    public function uninstall()
    {
        Configuration::deleteByName('HYPERSWITCH_LIVE_MODE');
        Configuration::deleteByName('HYPERSWITCH_API_KEY');
        Configuration::deleteByName('HYPERSWITCH_PUBLISHABLE_KEY');
        Configuration::deleteByName('HYPERSWITCH_WEBHOOK_SECRET');
        Configuration::deleteByName('PS_OS_HYPERSWITCH_AWAITING');
        Configuration::deleteByName('PS_OS_HYPERSWITCH_AUTHORIZED');

        // Delete order statuses
        $orderStateAwaiting = new OrderState(Configuration::get('PS_OS_HYPERSWITCH_AWAITING'));
        if (Validate::isLoadedObject($orderStateAwaiting)) {
            $orderStateAwaiting->deleted = true;
            $orderStateAwaiting->save();
        }

        $orderStateAuthorized = new OrderState(Configuration::get('PS_OS_HYPERSWITCH_AUTHORIZED'));
        if (Validate::isLoadedObject($orderStateAuthorized)) {
            $orderStateAuthorized->deleted = true;
            $orderStateAuthorized->save();
        }

        return parent::uninstall();
    }

    public function getContent()
    {
        if (((bool)Tools::isSubmit('submitHyperswitchModule')) == true) {
            $this->postProcess();
        }

        $this->context->smarty->assign('module_dir', $this->_path);

        $output = $this->context->smarty->fetch($this->local_path . 'views/templates/admin/configure.tpl');

        return $output . $this->renderForm();
    }

    protected function renderForm()
    {
        $helper = new HelperForm();

        $helper->show_toolbar = false;
        $helper->table = $this->table;
        $helper->module = $this;
        $helper->default_form_language = $this->context->language->id;
        $helper->allow_employee_form_lang = Configuration::get('PS_BO_ALLOW_EMPLOYEE_FORM_LANG', 0);

        $helper->identifier = $this->identifier;
        $helper->submit_action = 'submitHyperswitchModule';
        $helper->currentIndex = $this->context->link->getAdminLink('AdminModules', false)
            . '&configure=' . $this->name . '&tab_module=' . $this->tab . '&module_name=' . $this->name;
        $helper->token = Tools::getAdminTokenLite('AdminModules');

        $helper->tpl_vars = array(
            'fields_value' => $this->getConfigFormValues(),
            'languages' => $this->context->controller->getLanguages(),
            'id_language' => $this->context->language->id,
        );

        return $helper->generateForm(array($this->getConfigForm()));
    }

    protected function getConfigForm()
    {
        return array(
            'form' => array(
                'legend' => array(
                    'title' => $this->l('Settings'),
                    'icon' => 'icon-cogs',
                ),
                'input' => array(
                    array(
                        'type' => 'switch',
                        'label' => $this->l('Live mode'),
                        'name' => 'HYPERSWITCH_LIVE_MODE',
                        'is_bool' => true,
                        'desc' => $this->l('Use this module in live mode'),
                        'values' => array(
                            array(
                                'id' => 'active_on',
                                'value' => true,
                                'label' => $this->l('Enabled')
                            ),
                            array(
                                'id' => 'active_off',
                                'value' => false,
                                'label' => $this->l('Disabled')
                            )
                        ),
                    ),
                    array(
                        'col' => 3,
                        'type' => 'text',
                        'prefix' => '<i class="icon icon-key"></i>',
                        'desc' => $this->l('Enter your Hyperswitch API Key'),
                        'name' => 'HYPERSWITCH_API_KEY',
                        'label' => $this->l('API Key'),
                    ),
                    array(
                        'col' => 3,
                        'type' => 'text',
                        'prefix' => '<i class="icon icon-key"></i>',
                        'desc' => $this->l('Enter your Hyperswitch Publishable Key'),
                        'name' => 'HYPERSWITCH_PUBLISHABLE_KEY',
                        'label' => $this->l('Publishable Key'),
                    ),
                    array(
                        'col' => 3,
                        'type' => 'text',
                        'prefix' => '<i class="icon icon-key"></i>',
                        'desc' => $this->l('Enter your Hyperswitch Webhook Secret'),
                        'name' => 'HYPERSWITCH_WEBHOOK_SECRET',
                        'label' => $this->l('Webhook Secret'),
                    ),
                ),
                'submit' => array(
                    'title' => $this->l('Save'),
                ),
            ),
        );
    }

    protected function getConfigFormValues()
    {
        return array(
            'HYPERSWITCH_LIVE_MODE' => Configuration::get('HYPERSWITCH_LIVE_MODE', true),
            'HYPERSWITCH_API_KEY' => Configuration::get('HYPERSWITCH_API_KEY', ''),
            'HYPERSWITCH_PUBLISHABLE_KEY' => Configuration::get('HYPERSWITCH_PUBLISHABLE_KEY', ''),
            'HYPERSWITCH_WEBHOOK_SECRET' => Configuration::get('HYPERSWITCH_WEBHOOK_SECRET', ''),
        );
    }

    protected function postProcess()
    {
        $form_values = $this->getConfigFormValues();

        foreach (array_keys($form_values) as $key) {
            Configuration::updateValue($key, Tools::getValue($key));
        }
    }

    public function hookPaymentOptions($params)
    {
        if (!$this->active) {
            return;
        }

        if (!$this->checkCurrency($params['cart'])) {
            return;
        }

        $payment_options = [
            $this->getEmbeddedPaymentOption(),
        ];

        return $payment_options;
    }

    public function checkCurrency($cart)
    {
        $currency_order = new Currency($cart->id_currency);
        $currencies_module = $this->getCurrency($cart->id_currency);

        if (is_array($currencies_module)) {
            foreach ($currencies_module as $currency_module) {
                if ($currency_order->id == $currency_module['id_currency']) {
                    return true;
                }
            }
        }
        return false;
    }

    public function getEmbeddedPaymentOption()
    {
        $embeddedOption = new PrestaShop\PrestaShop\Core\Payment\PaymentOption();
        $embeddedOption->setCallToActionText($this->l('Pay with Hyperswitch'))
            ->setAction($this->context->link->getModuleLink($this->name, 'validation', array(), true))
            ->setAdditionalInformation($this->context->smarty->fetch('module:hyperswitch/views/templates/front/payment_infos.tpl'))
            ->setLogo(Media::getMediaPath(_PS_MODULE_DIR_ . $this->name . '/logo.png'));

        return $embeddedOption;
    }

    public function hookPaymentReturn($params)
    {
        if (!$this->active) {
            return;
        }

        $order = $params['order'];

        if ($order->getCurrentOrderState()->id != Configuration::get('PS_OS_ERROR')) {
            $this->smarty->assign('status', 'ok');
        }

        $this->smarty->assign(array(
            'id_order' => $order->id,
            'reference' => $order->reference,
            'params' => $params,
            'total' => Tools::displayPrice($params['order']->getOrdersTotalPaid(), new Currency($params['order']->id_currency), false),
        ));

        return $this->fetch('module:hyperswitch/views/templates/hook/payment_return.tpl');
    }

    public function hookHeader()
    {
        $this->context->controller->addCSS($this->_path . 'views/css/front.css');
    }

    public function hookBackOfficeHeader()
    {
        if (Tools::getValue('configure') == $this->name) {
            $this->context->controller->addCSS($this->_path . 'views/css/back.css');
        }
    }

    public function hookActionFrontControllerSetMedia()
    {
        if (Tools::getValue('controller') == 'order') {
            $this->context->controller->registerStylesheet(
                'module-hyperswitch-style',
                'modules/' . $this->name . '/views/css/front.css',
                array('media' => 'all', 'priority' => 150)
            );
            $this->context->controller->registerJavascript(
                'module-hyperswitch-js',
                'modules/' . $this->name . '/views/js/front.js',
                array('position' => 'bottom', 'priority' => 150)
            );
        }
    }

    public function processPayment($cartId, $amount, $currency, $customer)
    {
        $apiKey = Configuration::get('HYPERSWITCH_API_KEY');
        $baseUrl = Configuration::get('HYPERSWITCH_LIVE_MODE') ? 
            'https://api.hyperswitch.io' : 'https://sandbox.hyperswitch.io';

        $paymentData = [
            'amount' => $amount,
            'currency' => $currency,
            'customer_id' => $customer->id,
            'metadata' => [
                'cart_id' => $cartId,
                'prestashop_version' => _PS_VERSION_,
            ],
        ];

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $baseUrl . '/payments');
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);
        curl_setopt($ch, CURLOPT_POST, 1);
        curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($paymentData));
        curl_setopt($ch, CURLOPT_HTTPHEADER, [
            'Content-Type: application/json',
            'api-key: ' . $apiKey,
        ]);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($httpCode >= 200 && $httpCode < 300) {
            $result = json_decode($response, true);
            return $result;
        } else {
            PrestaShopLogger::addLog('Hyperswitch payment creation failed: ' . $response, 3);
            return false;
        }
    }

    public function capturePayment($paymentId, $amount = null)
    {
        $apiKey = Configuration::get('HYPERSWITCH_API_KEY');
        $baseUrl = Configuration::get('HYPERSWITCH_LIVE_MODE') ? 
            'https://api.hyperswitch.io' : 'https://sandbox.hyperswitch.io';

        $captureData = [];
        if ($amount !== null) {
            $captureData['amount'] = $amount;
        }

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $baseUrl . '/payments/' . $paymentId . '/capture');
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);
        curl_setopt($ch, CURLOPT_POST, 1);
        if (!empty($captureData)) {
            curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($captureData));
        }
        curl_setopt($ch, CURLOPT_HTTPHEADER, [
            'Content-Type: application/json',
            'api-key: ' . $apiKey,
        ]);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($httpCode >= 200 && $httpCode < 300) {
            $result = json_decode($response, true);
            return $result;
        } else {
            PrestaShopLogger::addLog('Hyperswitch payment capture failed: ' . $response, 3);
            return false;
        }
    }

    public function refundPayment($paymentId, $amount, $reason = '')
    {
        $apiKey = Configuration::get('HYPERSWITCH_API_KEY');
        $baseUrl = Configuration::get('HYPERSWITCH_LIVE_MODE') ? 
            'https://api.hyperswitch.io' : 'https://sandbox.hyperswitch.io';

        $refundData = [
            'amount' => $amount,
            'reason' => $reason,
        ];

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $baseUrl . '/payments/' . $paymentId . '/refunds');
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1);
        curl_setopt($ch, CURLOPT_POST, 1);
        curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($refundData));
        curl_setopt($ch, CURLOPT_HTTPHEADER, [
            'Content-Type: application/json',
            'api-key: ' . $apiKey,
        ]);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($httpCode >= 200 && $httpCode < 300) {
            $result = json_decode($response, true);
            return $result;
        } else {
            PrestaShopLogger::addLog('Hyperswitch payment refund failed: ' . $response, 3);
            return false;
        }
    }

    public function verifyWebhookSignature($payload, $signature)
    {
        $webhookSecret = Configuration::get('HYPERSWITCH_WEBHOOK_SECRET');
        $expectedSignature = hash_hmac('sha256', $payload, $webhookSecret);
        
        return hash_equals($expectedSignature, $signature);
    }
}
