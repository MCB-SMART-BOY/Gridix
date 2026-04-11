use super::types::{HelpAction, LearningTopic, LearningTopicStatus};
use super::*;
use crate::core::Action;
use crate::ui::{LocalShortcut, local_shortcut_text};
use egui::Stroke;

impl HelpDialog {
    pub(super) fn show_foundations_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "数据库、表、行、列分别是什么？",
            "先建立一个正确的心智模型，再学 SQL 才不会乱。",
        );

        Self::concept_card(
            ui,
            "核心概念",
            &[
                "数据库可以理解为一组有关联的数据集合。",
                "表是数据库里的一个主题区域，例如 customers、orders、payments。",
                "行是一条记录，列是这条记录的一个字段。",
                "学习数据库时，先学会“从表里读数据”，再学更复杂的关系和聚合。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先点下方“打开学习示例库”。",
                "2. 在左侧连接列表里选中 `Gridix 学习示例`。",
                "3. 在表列表里打开 `customers` 表。",
                "4. 再观察左侧：示例库现在包含客户、地址、供应商、分类、商品、订单、明细、支付 8 张主表。",
                "5. 观察结果区：每一行是一个客户，每一列是客户的属性。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "打开学习示例库",
                HelpAction::EnsureLearningSample { reset: false },
            )),
            Some((
                "自动查看 customers 表",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, customer_code, name, city, level, segment, reward_points, status FROM customers ORDER BY id LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开新建连接窗口", HelpAction::OpenConnectionDialog)),
        );
    }

    pub(super) fn show_data_types_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "数据类型：每一列为什么不能什么都塞",
            "数据库列有类型，不只是为了规范书写，更是为了约束存储和比较行为。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "常见类型包括整数、文本、浮点数、日期时间等。",
                "同一张表里，不同列通常表示不同含义，所以会有不同类型。",
                "类型会影响排序、比较和写入；例如数字和文本的比较方式不同。",
                "看懂类型，是理解表结构和写 INSERT / UPDATE 的前提。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开学习示例库里的 `products` 表。",
                "2. 执行 `PRAGMA table_info('products');` 查看列定义。",
                "3. 再执行 `SELECT id, name, price, cost, stock_qty, warranty_months, launch_date, typeof(price) AS price_type FROM products ORDER BY id LIMIT 5;`。",
                "4. 观察：同一张表里会同时出现文本、数值、整数、日期等多种类型。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动查看 products 列类型",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "PRAGMA table_info('products');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动演示 typeof(price)",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT id, name, price, cost, stock_qty, warranty_months, launch_date, typeof(price) AS price_type FROM products ORDER BY id LIMIT 5;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开学习示例库", HelpAction::EnsureLearningSample { reset: false })),
        );
    }

    pub(super) fn show_null_handling_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "NULL：缺失值不是空字符串，也不是 0",
            "很多数据库初学者的问题，不是 SQL 语法错，而是把 NULL 当成普通值来理解。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`NULL` 表示“当前没有值”或“未知”，不是空文本。",
                "判断 NULL 要用 `IS NULL` / `IS NOT NULL`，而不是 `= NULL`。",
                "NULL 经常出现在可选字段里，例如邮箱、发货时间、备注。",
                "学会处理 NULL，查询结果才不会漏掉或误判数据。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `customers` 表，执行 `SELECT id, name, email FROM customers WHERE email IS NULL;`。",
                "2. 再执行 `SELECT id, status, shipped_at FROM orders WHERE shipped_at IS NULL ORDER BY id;`。",
                "3. 观察哪些记录因为“还没有值”而显示为空。",
                "4. 再尝试把 `IS NULL` 改成 `IS NOT NULL`，比较结果差异。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示 email 的 NULL 查询",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, email FROM customers WHERE email IS NULL ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动演示 shipped_at 的 NULL 查询",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT id, status, shipped_at FROM orders WHERE shipped_at IS NULL ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("重置学习示例库", HelpAction::EnsureLearningSample { reset: true })),
        );
    }

    pub(super) fn show_select_topic(
        ui: &mut egui::Ui,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let toggle_editor = Self::topic_binding(context, Action::ToggleEditor, "Ctrl+J");
        let sql_execute = local_shortcut_text(LocalShortcut::SqlExecute);
        Self::topic_header(
            ui,
            "SELECT 基础：从表里读取你需要的列",
            "数据库学习的第一步，不是修改数据，而是读懂数据。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`SELECT` 用来取数据。",
                "`FROM` 指定从哪张表取数据。",
                "`LIMIT` 控制先看多少行，适合新手避免结果太长。",
                "一次只挑 2 到 4 列最容易观察数据结构。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `customers` 表。",
                &format!("2. 按 {toggle_editor} 打开 SQL 编辑器。"),
                "3. 输入 `SELECT id, name, city FROM customers LIMIT 5;`。",
                &format!("4. 按 {sql_execute}，看结果区是否出现 5 条客户记录。"),
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示 SELECT",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, city FROM customers ORDER BY id LIMIT 5;".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
            None,
        );
    }

    pub(super) fn show_like_topic(ui: &mut egui::Ui, pending_ui_action: &mut Option<HelpUiAction>) {
        Self::topic_header(
            ui,
            "LIKE：在文本里按关键字模糊匹配",
            "当你记不住完整值，只知道一部分文本时，LIKE 是最直接的入口。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`LIKE` 常用于文本列匹配。",
                "`%` 表示任意长度字符，`_` 表示单个字符。",
                "`LIKE '%ing%'` 的意思是“包含 ing 这段文本”。",
                "模糊匹配适合搜索，但通常比精准条件更宽，所以更要注意结果范围。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `customers` 表。",
                "2. 执行 `SELECT id, name, city FROM customers WHERE city LIKE '%ing%' ORDER BY id;`。",
                "3. 观察哪些城市名里包含 `ing`。",
                "4. 再执行 `SELECT id, name FROM products WHERE name LIKE '%Mouse%';`，体验另一种文本搜索。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示城市 LIKE 查询",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, city FROM customers WHERE city LIKE '%ing%' ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动演示商品名 LIKE 查询",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT id, name, category FROM products WHERE name LIKE '%Mouse%' ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            None,
        );
    }

    pub(super) fn show_filter_sort_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "WHERE 与 ORDER BY：筛选你想看的数据，再排序",
            "真实数据库查询通常不是“全表扫一遍”，而是先筛再排。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`WHERE` 用来筛掉不需要的行。",
                "`ORDER BY` 决定结果呈现顺序。",
                "`DESC` 表示从大到小，`ASC` 表示从小到大。",
                "筛选与排序组合后，才是日常工作里最常用的查询。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `products` 表。",
                "2. 在编辑器输入 `SELECT id, name, category, price, stock_qty, rating FROM products WHERE price >= 200 AND stock_qty >= 40 ORDER BY rating DESC, price DESC LIMIT 8;`。",
                "3. 执行后观察：结果只保留高价且库存充足的商品，并按评分和价格排序。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示筛选与排序",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT id, name, category, price, stock_qty, rating FROM products WHERE price >= 200 AND stock_qty >= 40 ORDER BY rating DESC, price DESC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开学习示例库", HelpAction::EnsureLearningSample { reset: false })),
            None,
        );
    }

    pub(super) fn show_aggregate_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "GROUP BY：从明细数据里提炼出统计结论",
            "数据库不仅能列记录，还能帮你总结规律。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`COUNT` 用来计数，`SUM` 用来求和。",
                "`GROUP BY` 决定按什么维度汇总。",
                "只看明细时你看到的是“发生了什么”，做聚合后你看到的是“整体规律”。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `products` 或 `product_categories` 表。",
                "2. 输入 `SELECT pc.department, COUNT(*) AS product_count, ROUND(AVG(p.price), 2) AS avg_price, ROUND(SUM(p.stock_qty * p.cost), 2) AS inventory_cost FROM products p JOIN product_categories pc ON pc.id = p.category_id GROUP BY pc.department ORDER BY inventory_cost DESC LIMIT 8;`。",
                "3. 执行后观察：每个业务部门有多少商品、均价和库存成本大概是多少。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示 GROUP BY",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT pc.department, COUNT(*) AS product_count, ROUND(AVG(p.price), 2) AS avg_price, ROUND(SUM(p.stock_qty * p.cost), 2) AS inventory_cost FROM products p JOIN product_categories pc ON pc.id = p.category_id GROUP BY pc.department ORDER BY inventory_cost DESC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("重置学习示例库", HelpAction::EnsureLearningSample { reset: true })),
            None,
        );
    }

    pub(super) fn show_relationships_topic(
        ui: &mut egui::Ui,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let toggle_er_diagram = Self::topic_binding(context, Action::ToggleErDiagram, "Ctrl+R");
        Self::topic_header(
            ui,
            "主键、外键、关系图：理解表为什么能连起来",
            "如果不理解主键和外键，JOIN 只是会写，不算真正理解关系型数据库。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "主键是每一行的唯一标识，例如 `customers.id`。",
                "外键指向另一张表的主键，例如 `orders.customer_id` 指向 `customers.id`。",
                "ER 图把这些关系用图形方式表现出来，非常适合新手建立全局理解。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开学习示例库。",
                &format!("2. 按 {toggle_er_diagram} 打开 ER 图。"),
                "3. 找到 `customers -> orders -> order_items -> products -> suppliers` 这条关系链。",
                "4. 再找 `customers -> customer_addresses` 和 `product_categories -> products` 两条关系，观察一对多与层级关系的区别。",
                "5. 执行 `PRAGMA foreign_key_list('orders');` 与 `PRAGMA foreign_key_list('products');`，观察外键具体指向哪张表。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some(("自动打开学习示例 ER 图", HelpAction::ShowLearningErDiagram)),
            Some((
                "自动查看 orders 外键",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "PRAGMA foreign_key_list('orders');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动查看 products 外键",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "PRAGMA foreign_key_list('products');".to_string(),
                    open_er_diagram: false,
                },
            )),
        );
    }

    pub(super) fn show_join_topic(ui: &mut egui::Ui, pending_ui_action: &mut Option<HelpUiAction>) {
        Self::topic_header(
            ui,
            "JOIN：把分散在不同表里的信息拼起来",
            "关系型数据库最重要的价值之一，就是通过外键和 JOIN 组合数据。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "订单和客户通常不会放在一张超大表里。",
                "`orders.customer_id = customers.id` 这样的字段关系，就是表之间的连接点。",
                "`JOIN` 允许你在一张结果里同时看到客户和订单信息。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先理解 `orders` 保存订单，`customers` 保存客户，`order_items` 保存每个订单里的商品明细，`products` 保存商品本身。",
                "2. 在编辑器输入 `SELECT o.id AS order_id, c.name AS customer, p.name AS product, oi.quantity, o.status, pay.status AS payment_status FROM orders o JOIN customers c ON c.id = o.customer_id JOIN order_items oi ON oi.order_id = o.id JOIN products p ON p.id = oi.product_id JOIN payments pay ON pay.order_id = o.id ORDER BY o.id DESC, oi.line_number ASC LIMIT 8;`。",
                "3. 执行后观察：客户、订单、商品、支付状态已经被拼成一张结果表。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示 JOIN",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT o.id AS order_id, c.name AS customer, p.name AS product, oi.quantity, o.status, pay.status AS payment_status FROM orders o JOIN customers c ON c.id = o.customer_id JOIN order_items oi ON oi.order_id = o.id JOIN products p ON p.id = oi.product_id JOIN payments pay ON pay.order_id = o.id ORDER BY o.id DESC, oi.line_number ASC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "打开 ER 图辅助理解",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT o.id AS order_id, c.name AS customer, p.name AS product, oi.quantity, o.status, pay.status AS payment_status FROM orders o JOIN customers c ON c.id = o.customer_id JOIN order_items oi ON oi.order_id = o.id JOIN products p ON p.id = oi.product_id JOIN payments pay ON pay.order_id = o.id ORDER BY o.id DESC, oi.line_number ASC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: true,
                },
            )),
            None,
        );
    }

    pub(super) fn show_insert_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "INSERT：向表里新增一条记录",
            "写入数据之前，先确认要写入哪张表、哪些列，以及值的顺序是否对应。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`INSERT INTO table (列...) VALUES (值...)` 用来新增一行数据。",
                "显式写出列名，比只写 `VALUES (...)` 更安全，也更适合新手。",
                "插入的数据必须和列定义匹配，例如文本列要给文本，数值列要给数值。",
                "写操作会改变数据库状态，所以学习时最好先在示例库中练习。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先打开学习示例库，选中 `customers` 表。",
                "2. 在编辑器输入 `INSERT INTO customers (id, name, city, level) VALUES (1001, 'Grace He', 'Suzhou', 'Silver');`。",
                "3. 执行后，再运行 `SELECT id, name, city, level FROM customers ORDER BY id DESC LIMIT 3;`。",
                "4. 观察结果区：新增客户已经出现在表中。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示 INSERT",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql: "INSERT INTO customers (id, name, city, level) VALUES (1001, 'Grace He', 'Suzhou', 'Silver');"
                        .to_string(),
                    preview_table: Some("customers".to_string()),
                    preview_sql:
                        "SELECT id, name, city, level FROM customers ORDER BY id DESC LIMIT 3;"
                            .to_string(),
                    success_message: "INSERT 演示已完成，已为学习示例库新增一条客户记录。"
                        .to_string(),
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
            None,
        );
    }

    pub(super) fn show_constraints_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "约束：数据库用什么规则保护数据质量",
            "约束不是语法装饰，而是数据库层面最重要的自我保护机制之一。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`PRIMARY KEY` 保证唯一标识。",
                "`NOT NULL` 要求这一列必须有值，`DEFAULT` 提供默认值。",
                "`FOREIGN KEY` 让表之间的关系真正被数据库认识。",
                "约束的价值在于：即使应用代码写错，数据库也能拦住一部分坏数据。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先执行 `PRAGMA table_info('customers');`，观察哪些列不允许为空、哪些列带默认值。",
                "2. 再执行 `PRAGMA foreign_key_list('orders');`，观察订单表如何指向客户与地址表。",
                "3. 最后执行 `PRAGMA foreign_key_list('payments');`，观察支付表如何挂到订单表上。",
                "4. 如果想更直观，再打开 ER 图，把图形关系和外键信息对上。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动查看 customers 约束",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "PRAGMA table_info('customers');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动查看 orders 外键",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "PRAGMA foreign_key_list('orders');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动查看 payments 外键",
                HelpAction::RunLearningQuery {
                    table: Some("payments".to_string()),
                    sql: "PRAGMA foreign_key_list('payments');".to_string(),
                    open_er_diagram: false,
                },
            )),
        );
    }

    pub(super) fn show_update_delete_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "UPDATE 与 DELETE：先筛选，再修改或删除",
            "真正危险的不是写操作本身，而是不带条件地改整张表。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`UPDATE` 修改已有记录，`DELETE` 删除已有记录。",
                "这两类语句几乎都应该先配合 `WHERE` 使用，否则容易误改整张表。",
                "在真实环境里，最好先写一条 `SELECT ... WHERE ...` 预览受影响的行。",
                "学习时可以随时重置示例库，所以这里适合反复练习。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先运行 `SELECT id, status FROM orders WHERE id = 1004;`，确认要修改的是哪一行。",
                "2. 再执行 `UPDATE orders SET status = 'SHIPPED' WHERE id = 1004;`。",
                "3. 然后执行 `SELECT id, status FROM orders WHERE id = 1004;`，观察状态是否变化。",
                "4. 如果要练习删除，先重置示例库，再执行 `DELETE FROM orders WHERE id = 1006;`，最后同时检查订单、订单明细、支付记录是否都被移除。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示 UPDATE",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql: "UPDATE orders SET status = 'SHIPPED' WHERE id = 1004;"
                        .to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql: "SELECT id, status, total_amount FROM orders WHERE id = 1004;"
                        .to_string(),
                    success_message: "UPDATE 演示已完成，订单 1004 的状态已更新。".to_string(),
                },
            )),
            Some((
                "自动演示 DELETE",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql: "DELETE FROM orders WHERE id = 1006;".to_string(),
                    preview_table: None,
                    preview_sql:
                        "SELECT (SELECT COUNT(*) FROM orders WHERE id = 1006) AS orders_left, (SELECT COUNT(*) FROM order_items WHERE order_id = 1006) AS order_items_left, (SELECT COUNT(*) FROM payments WHERE order_id = 1006) AS payments_left;"
                            .to_string(),
                    success_message: "DELETE 演示已完成，订单 1006 及其关联明细、支付记录都已从学习示例库移除。"
                        .to_string(),
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
        );
    }

    pub(super) fn show_transactions_topic(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        Self::topic_header(
            ui,
            "事务：一批操作为什么要么全成功、要么全撤销",
            "事务是数据库最关键的安全能力之一，它保护的是“多步修改”的一致性。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`BEGIN` 表示事务开始，`COMMIT` 表示提交，`ROLLBACK` 表示撤销。",
                "当几步修改必须一起成功时，事务能避免“只改到一半”的中间状态。",
                "新手最重要的习惯不是背 ACID，而是先知道事务能保护写操作。",
                "在学习示例库中，你可以安全地演示提交和回滚。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先查看 `SELECT o.id, o.status, o.payment_status, p.status AS payment_record_status, p.paid_at FROM orders o JOIN payments p ON p.order_id = o.id WHERE o.id = 1004;`。",
                "2. 执行 `BEGIN; UPDATE orders SET status = 'PAID', payment_status = 'PAID' WHERE id = 1004; UPDATE payments SET status = 'CAPTURED', paid_at = '2026-04-01 10:00:00', settled_at = '2026-04-01 12:00:00' WHERE order_id = 1004; ROLLBACK;`。",
                "3. 再查一次同一订单和支付记录，观察两张表都没有被改动。",
                "4. 如果把 `ROLLBACK` 换成 `COMMIT`，订单状态和支付状态才会一起真正保留下来。",
            ],
        );

        Self::action_row(
            ui,
            pending_ui_action,
            Some((
                "自动演示事务回滚",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql:
                        "BEGIN;\nUPDATE orders SET status = 'PAID', payment_status = 'PAID' WHERE id = 1004;\nUPDATE payments SET status = 'CAPTURED', paid_at = '2026-04-01 10:00:00', settled_at = '2026-04-01 12:00:00' WHERE order_id = 1004;\nROLLBACK;"
                            .to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql: "SELECT o.id, o.status, o.payment_status, p.status AS payment_record_status, p.paid_at FROM orders o JOIN payments p ON p.order_id = o.id WHERE o.id = 1004;"
                        .to_string(),
                    success_message:
                        "事务回滚演示已完成，订单 1004 与其支付记录都保持原始状态。"
                            .to_string(),
                },
            )),
            Some((
                "自动演示事务提交",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql:
                        "BEGIN;\nUPDATE orders SET status = 'PAID', payment_status = 'PAID' WHERE id = 1004;\nUPDATE payments SET status = 'CAPTURED', paid_at = '2026-04-01 10:00:00', settled_at = '2026-04-01 12:00:00' WHERE order_id = 1004;\nCOMMIT;"
                            .to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql: "SELECT o.id, o.status, o.payment_status, p.status AS payment_record_status, p.paid_at FROM orders o JOIN payments p ON p.order_id = o.id WHERE o.id = 1004;"
                        .to_string(),
                    success_message:
                        "事务提交演示已完成，订单 1004 与其支付记录已被一致地更新。"
                            .to_string(),
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
        );
    }

    pub(super) fn show_roadmap_preview_topic(ui: &mut egui::Ui, topic: LearningTopic) {
        Self::topic_header(
            ui,
            Self::topic_title(topic),
            "这个知识点已经放进完整路线图里，但当前阶段先展示它的位置、价值和前置依赖。",
        );

        Self::concept_card(
            ui,
            "为什么它重要",
            &[
                Self::topic_definition(topic).summary,
                Self::topic_definition(topic).dependency_text,
                Self::topic_definition(topic).follow_up_text,
            ],
        );

        let preview_hint = match Self::topic_definition(topic).status {
            LearningTopicStatus::Planned => {
                "这是下一阶段会逐步补齐的主题，后续会增加示例、练习和自动演示。"
            }
            LearningTopicStatus::Advanced => {
                "这是进阶主题，先知道它存在和依赖关系即可，不建议现在跳过去硬学。"
            }
            LearningTopicStatus::Available => "这个主题已经可以学习。",
        };

        Self::practice_card(
            ui,
            "当前建议",
            &[
                preview_hint,
                "先完成前置知识点，再回到这里继续推进整条学习路线。",
                "如果你只是想建立全局认知，这一页已经足够告诉你它为什么重要。",
            ],
        );
    }

    fn topic_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(95, 125, 180, 18))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 36),
            ))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(title)
                        .size(19.0)
                        .strong()
                        .color(Color32::from_rgb(130, 180, 255)),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(subtitle).color(Self::muted_text_color(ui)));
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    Self::step_chip(ui, "先理解概念");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "再手动练习");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "不会时点自动演示");
                });
            });
        ui.add_space(14.0);
    }

    fn concept_card(ui: &mut egui::Ui, title: &str, items: &[&str]) {
        Self::info_card(
            ui,
            title,
            items,
            InfoCardStyle {
                section_label: "理解概念",
                intro: "先把概念和边界想清楚，再去下面实际操作。",
                fill: Color32::from_rgba_unmultiplied(90, 140, 210, 20),
                stroke: Color32::from_rgba_unmultiplied(120, 170, 230, 44),
                accent: Color32::from_rgb(130, 180, 255),
            },
        );
        ui.add_space(12.0);
    }

    fn practice_card(ui: &mut egui::Ui, title: &str, items: &[&str]) {
        Self::info_card(
            ui,
            title,
            items,
            InfoCardStyle {
                section_label: "动手练习",
                intro: "按顺序操作；如果卡住了，直接用下面的自动演示验证。",
                fill: Color32::from_rgba_unmultiplied(92, 180, 118, 18),
                stroke: Color32::from_rgba_unmultiplied(100, 190, 126, 40),
                accent: Color32::from_rgb(146, 214, 160),
            },
        );
        ui.add_space(12.0);
    }

    fn info_card(ui: &mut egui::Ui, title: &str, items: &[&str], style: InfoCardStyle<'_>) {
        let InfoCardStyle {
            section_label,
            intro,
            fill,
            stroke,
            accent,
        } = style;
        let width = ui.available_width();
        egui::Frame::NONE
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_min_width((width - 32.0).max(260.0));
                ui.set_max_width((width - 32.0).max(260.0));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(
                            accent.r(),
                            accent.g(),
                            accent.b(),
                            26,
                        ))
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 46),
                        ))
                        .corner_radius(egui::CornerRadius::same(255))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.label(RichText::new(section_label).small().strong().color(accent));
                        });
                    ui.label(
                        RichText::new(title)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(224, 228, 236)),
                    );
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(intro)
                        .small()
                        .color(Color32::from_rgb(176, 180, 190)),
                );
                ui.add_space(10.0);
                for item in items {
                    Self::topic_card_item(ui, item, accent);
                    ui.add_space(6.0);
                }
            });
    }

    fn topic_card_item(ui: &mut egui::Ui, item: &str, accent: Color32) {
        if let Some((step_no, text)) = Self::split_step_item(item) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        28,
                    ))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 44),
                    ))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.label(RichText::new(step_no).small().strong().color(accent));
                    });
                ui.add(
                    egui::Label::new(RichText::new(text).color(Color32::from_rgb(204, 208, 216)))
                        .wrap(),
                );
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            ui.label(RichText::new("•").strong().color(accent));
            ui.add(
                egui::Label::new(RichText::new(item).color(Color32::from_rgb(204, 208, 216)))
                    .wrap(),
            );
        });
    }

    fn split_step_item(item: &str) -> Option<(&str, &str)> {
        let trimmed = item.trim_start();
        let digits_len = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();

        if digits_len == 0 {
            return None;
        }

        let bytes = trimmed.as_bytes();
        if bytes.get(digits_len) != Some(&b'.') || bytes.get(digits_len + 1) != Some(&b' ') {
            return None;
        }

        Some((&trimmed[..digits_len], &trimmed[(digits_len + 2)..]))
    }

    fn action_row(
        ui: &mut egui::Ui,
        pending_ui_action: &mut Option<HelpUiAction>,
        primary: Option<(&str, HelpAction)>,
        secondary: Option<(&str, HelpAction)>,
        tertiary: Option<(&str, HelpAction)>,
    ) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(88, 108, 150, 14))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 28),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("直接在 Gridix 里试一遍")
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("不会做时先点自动演示；想自己练时再切回编辑器手动操作。")
                        .small()
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(10.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

                    if let Some((label, value)) = primary
                        && Self::action_button(ui, label, true)
                    {
                        *pending_ui_action = Some(HelpUiAction::Dispatch(value));
                    }
                    if let Some((label, value)) = secondary
                        && Self::action_button(ui, label, false)
                    {
                        *pending_ui_action = Some(HelpUiAction::Dispatch(value));
                    }
                    if let Some((label, value)) = tertiary
                        && Self::action_button(ui, label, false)
                    {
                        *pending_ui_action = Some(HelpUiAction::Dispatch(value));
                    }
                });
            });

        ui.add_space(12.0);
    }

    pub(super) fn action_button(ui: &mut egui::Ui, label: &str, primary: bool) -> bool {
        let fill = if primary {
            Color32::from_rgb(60, 112, 190)
        } else {
            Color32::from_rgba_unmultiplied(120, 120, 130, 28)
        };
        let stroke = if primary {
            Color32::from_rgba_unmultiplied(150, 205, 255, 48)
        } else {
            Color32::from_rgba_unmultiplied(170, 176, 194, 24)
        };

        ui.add(
            egui::Button::new(
                RichText::new(label)
                    .strong()
                    .color(Color32::from_rgb(245, 245, 248)),
            )
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(8)),
        )
        .clicked()
    }

    fn topic_binding(context: &HelpContext, action: Action, fallback: &str) -> String {
        let binding = context.keybindings.display(action);
        if binding.is_empty() {
            fallback.to_owned()
        } else {
            binding
        }
    }
}

struct InfoCardStyle<'a> {
    section_label: &'a str,
    intro: &'a str,
    fill: Color32,
    stroke: Color32,
    accent: Color32,
}
