INSERT INTO `PREFIX_order_state` (`invoice`, `send_email`, `module_name`, `color`, `unremovable`, `hidden`, `logable`, `delivery`, `shipped`, `paid`, `pdf_invoice`, `pdf_delivery`, `deleted`) 
VALUES 
(0, 0, 'hyperswitch', '#34209E', 1, 0, 0, 0, 0, 0, 0, 0, 0),
(0, 0, 'hyperswitch', '#4169E1', 1, 0, 0, 0, 0, 0, 0, 0, 0);

INSERT INTO `PREFIX_order_state_lang` (`id_order_state`, `id_lang`, `name`, `template`) 
SELECT os.`id_order_state`, l.`id_lang`, 
CASE 
    WHEN os.`color` = '#34209E' THEN 'Awaiting Hyperswitch Payment'
    WHEN os.`color` = '#4169E1' THEN 'Hyperswitch Authorized'
END,
''
FROM `PREFIX_order_state` os
CROSS JOIN `PREFIX_lang` l
WHERE os.`module_name` = 'hyperswitch'
AND (os.`color` = '#34209E' OR os.`color` = '#4169E1');
