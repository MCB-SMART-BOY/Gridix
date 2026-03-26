#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTab {
    #[default]
    ToolQuickStart,
    DatabaseLearning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LearningTopic {
    #[default]
    Foundations,
    DataTypes,
    NullHandling,
    SelectBasics,
    FilterAndSort,
    LikePattern,
    Aggregate,
    Relationships,
    Join,
    InsertData,
    Constraints,
    UpdateDelete,
    Transactions,
    SchemaDesign,
    Views,
    Indexes,
    Subqueries,
    WindowFunctions,
    TriggersProcedures,
    QueryPlans,
    BackupPermissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LearningView {
    #[default]
    Overview,
    Roadmap,
    TopicDetail,
}

#[derive(Debug, Clone, Default)]
pub struct HelpState {
    pub(crate) active_tab: HelpTab,
    pub(crate) learning_view: LearningView,
    pub(crate) learning_topic: LearningTopic,
}

#[derive(Debug, Clone, Default)]
pub struct HelpContext {
    pub active_connection_name: Option<String>,
    pub selected_table: Option<String>,
    pub has_result: bool,
    pub show_sql_editor: bool,
    pub show_er_diagram: bool,
}

#[derive(Debug, Clone)]
pub enum HelpAction {
    OpenConnectionDialog,
    EnsureLearningSample {
        reset: bool,
    },
    RunLearningQuery {
        table: Option<String>,
        sql: String,
        open_er_diagram: bool,
    },
    RunLearningMutationDemo {
        reset: bool,
        mutation_sql: String,
        preview_table: Option<String>,
        preview_sql: String,
        success_message: String,
    },
    ShowLearningErDiagram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LearningStage {
    Fundamentals,
    QueryBasics,
    RelationshipModel,
    Mutations,
    DesignQuality,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LearningTopicStatus {
    Available,
    Planned,
    Advanced,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LearningTopicDefinition {
    pub topic: LearningTopic,
    pub stage: LearningStage,
    pub status: LearningTopicStatus,
    pub title: &'static str,
    pub short_title: &'static str,
    pub summary: &'static str,
    pub dependency_text: &'static str,
    pub follow_up_text: &'static str,
}

pub(super) const TOPIC_DEFINITIONS: [LearningTopicDefinition; 21] = [
    LearningTopicDefinition {
        topic: LearningTopic::Foundations,
        stage: LearningStage::Fundamentals,
        status: LearningTopicStatus::Available,
        title: "数据库、表、行、列",
        short_title: "数据库 / 表 / 行 / 列",
        summary: "建立数据库的最小心智模型，知道数据库、表、行、列分别是什么。",
        dependency_text: "前置要求：无。建议所有人从这里开始。",
        follow_up_text: "下一步建议看数据类型与 NULL，把结构概念补完整。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::DataTypes,
        stage: LearningStage::Fundamentals,
        status: LearningTopicStatus::Available,
        title: "数据类型",
        short_title: "数据类型",
        summary: "理解文本、数字、日期等类型为什么会影响存储、比较和写入。",
        dependency_text: "前置要求：先理解表、行、列。",
        follow_up_text: "接着学习 NULL 和 SELECT，开始读取真实数据。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::NullHandling,
        stage: LearningStage::Fundamentals,
        status: LearningTopicStatus::Available,
        title: "NULL 与空值",
        short_title: "NULL",
        summary: "理解 NULL 不是空字符串，也不是 0，并学会用 IS NULL 判断缺失值。",
        dependency_text: "前置要求：先理解数据类型和列的含义。",
        follow_up_text: "接着去学 SELECT 和 WHERE，把 NULL 放进查询条件里。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::SelectBasics,
        stage: LearningStage::QueryBasics,
        status: LearningTopicStatus::Available,
        title: "SELECT 基础",
        short_title: "SELECT",
        summary: "学会用 SELECT / FROM / LIMIT 从一张表中读取并观察数据。",
        dependency_text: "前置要求：先理解数据库、表、行、列和基本数据类型。",
        follow_up_text: "下一步建议看 WHERE 与 ORDER BY，开始控制查询结果。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::FilterAndSort,
        stage: LearningStage::QueryBasics,
        status: LearningTopicStatus::Available,
        title: "WHERE 与 ORDER BY",
        short_title: "WHERE / ORDER BY",
        summary: "学会先筛选再排序，这是日常查询最常见的组合。",
        dependency_text: "前置要求：先会最基本的 SELECT。",
        follow_up_text: "下一步可以去看 LIKE、GROUP BY 或 UPDATE/DELETE 的安全前置习惯。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::LikePattern,
        stage: LearningStage::QueryBasics,
        status: LearningTopicStatus::Available,
        title: "LIKE 模糊匹配",
        short_title: "LIKE",
        summary: "理解通配符匹配，学会从文本列里按关键字搜索。",
        dependency_text: "前置要求：先会 WHERE，理解字符串条件查询。",
        follow_up_text: "下一步建议去 GROUP BY 或 JOIN，学习更复杂的结果组织方式。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Aggregate,
        stage: LearningStage::QueryBasics,
        status: LearningTopicStatus::Available,
        title: "GROUP BY 聚合",
        short_title: "GROUP BY",
        summary: "学会从明细记录中提炼统计结论，例如计数、求和和分组。",
        dependency_text: "前置要求：先会 SELECT 和基本筛选。",
        follow_up_text: "下一步建议理解表关系和 JOIN，再把统计和关系查询结合起来。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Relationships,
        stage: LearningStage::RelationshipModel,
        status: LearningTopicStatus::Available,
        title: "主键、外键、关系",
        short_title: "主键 / 外键 / 关系",
        summary: "理解主键、外键和表之间的关系，为 JOIN 和 ER 图打基础。",
        dependency_text: "前置要求：先理解表和主键，再看外键关系。",
        follow_up_text: "下一步去看 JOIN，把表关系真正用在查询里。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Join,
        stage: LearningStage::RelationshipModel,
        status: LearningTopicStatus::Available,
        title: "JOIN 关联查询",
        short_title: "JOIN",
        summary: "学会把分散在不同表中的信息按关系拼接成一张结果表。",
        dependency_text: "前置要求：先理解表关系，再会筛选和排序。",
        follow_up_text: "下一步可以看视图、子查询和查询计划，理解复杂查询如何组织。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::InsertData,
        stage: LearningStage::Mutations,
        status: LearningTopicStatus::Available,
        title: "INSERT 新增数据",
        short_title: "INSERT",
        summary: "学会安全地新增一条记录，理解写入时列和值必须匹配。",
        dependency_text: "前置要求：先理解表结构，并会最基本的 SELECT。",
        follow_up_text: "下一步去看约束和 UPDATE/DELETE，理解写入后的规则与修改风险。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Constraints,
        stage: LearningStage::Mutations,
        status: LearningTopicStatus::Available,
        title: "约束与默认值",
        short_title: "约束",
        summary: "理解 PRIMARY KEY、NOT NULL、UNIQUE、DEFAULT 和 FOREIGN KEY 如何保护数据质量。",
        dependency_text: "前置要求：先理解主键、外键，并知道 INSERT 会真正写入数据。",
        follow_up_text: "下一步建议去学 UPDATE/DELETE 和事务，体会约束如何保护修改过程。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::UpdateDelete,
        stage: LearningStage::Mutations,
        status: LearningTopicStatus::Available,
        title: "UPDATE 与 DELETE",
        short_title: "UPDATE / DELETE",
        summary: "学会带条件地更新或删除数据，建立“先筛选、后修改”的习惯。",
        dependency_text: "前置要求：先会 WHERE，并理解写入操作会真实改变数据。",
        follow_up_text: "下一步去看事务，理解一批修改如何一起提交或回滚。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Transactions,
        stage: LearningStage::Mutations,
        status: LearningTopicStatus::Available,
        title: "事务",
        short_title: "事务",
        summary: "理解一组操作为什么要么全部成功、要么全部撤销。",
        dependency_text: "前置要求：先理解 INSERT、UPDATE、DELETE 的写入含义。",
        follow_up_text: "下一步可以进入表设计、权限与备份恢复这些更靠近真实环境的话题。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::SchemaDesign,
        stage: LearningStage::DesignQuality,
        status: LearningTopicStatus::Planned,
        title: "表设计与规范化",
        short_title: "表设计",
        summary: "理解为什么一张表应表达一个主题，以及什么时候需要拆表。",
        dependency_text: "前置要求：先理解主键、外键、约束和事务。",
        follow_up_text: "后续会结合视图和索引，讲如何让结构更清晰也更可维护。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Views,
        stage: LearningStage::DesignQuality,
        status: LearningTopicStatus::Planned,
        title: "视图",
        short_title: "视图",
        summary: "理解视图如何把常用查询封装成可复用的数据入口。",
        dependency_text: "前置要求：先会 SELECT、JOIN 和聚合。",
        follow_up_text: "后续会与表设计一起讲，帮助你区分“真实存储”和“查询视角”。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Indexes,
        stage: LearningStage::DesignQuality,
        status: LearningTopicStatus::Planned,
        title: "索引",
        short_title: "索引",
        summary: "理解索引解决的是查询速度问题，以及为什么索引不是越多越好。",
        dependency_text: "前置要求：先会筛选、排序和常见查询模式。",
        follow_up_text: "后续会和查询计划一起讲，帮助你建立性能判断的基础。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::Subqueries,
        stage: LearningStage::Advanced,
        status: LearningTopicStatus::Advanced,
        title: "子查询",
        short_title: "子查询",
        summary: "理解查询里再嵌一层查询时，何时清晰、何时容易失控。",
        dependency_text: "前置要求：先熟悉 SELECT、JOIN 和聚合。",
        follow_up_text: "它常常和视图、窗口函数、查询计划一起出现。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::WindowFunctions,
        stage: LearningStage::Advanced,
        status: LearningTopicStatus::Advanced,
        title: "窗口函数",
        short_title: "窗口函数",
        summary: "理解排名、累计、分组内计算等分析型查询的核心能力。",
        dependency_text: "前置要求：先会 GROUP BY，并理解分组与明细行的区别。",
        follow_up_text: "后续会作为进阶分析能力讲解，不建议在入门阶段提前跳进去。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::TriggersProcedures,
        stage: LearningStage::Advanced,
        status: LearningTopicStatus::Advanced,
        title: "触发器与存储过程",
        short_title: "触发器 / 存储过程",
        summary: "理解数据库内部自动化逻辑适合放在哪里，以及它们的维护成本。",
        dependency_text: "前置要求：先理解约束、事务和基本表设计。",
        follow_up_text: "后续会作为进阶主题，和权限、审计、业务规则放在一起讲。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::QueryPlans,
        stage: LearningStage::Advanced,
        status: LearningTopicStatus::Advanced,
        title: "查询计划",
        short_title: "查询计划",
        summary: "理解数据库为什么会选择某种执行方式，以及它和索引的关系。",
        dependency_text: "前置要求：先理解索引、JOIN 和常见筛选排序模式。",
        follow_up_text: "这是性能优化入口，放在主干课完成之后更合适。",
    },
    LearningTopicDefinition {
        topic: LearningTopic::BackupPermissions,
        stage: LearningStage::Advanced,
        status: LearningTopicStatus::Advanced,
        title: "备份、恢复与权限",
        short_title: "备份 / 恢复 / 权限",
        summary: "理解真实数据库环境里如何保护数据、恢复数据和限制访问。",
        dependency_text: "前置要求：先理解事务、写操作风险和真实环境的安全边界。",
        follow_up_text: "这是从“会写 SQL”走向“能安全使用数据库”的关键一步。",
    },
];
