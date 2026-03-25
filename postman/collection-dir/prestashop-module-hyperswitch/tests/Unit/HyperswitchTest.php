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

use PHPUnit\Framework\TestCase;

class HyperswitchTest extends TestCase
{
    public function testModuleName()
    {
        $module = new Hyperswitch();
        $this->assertEquals('hyperswitch', $module->name);
    }

    public function testModuleVersion()
    {
        $module = new Hyperswitch();
        $this->assertEquals('1.0.0', $module->version);
    }

    public function testModuleAuthor()
    {
        $module = new Hyperswitch();
        $this->assertEquals('Juspay Technologies', $module->author);
    }
}
