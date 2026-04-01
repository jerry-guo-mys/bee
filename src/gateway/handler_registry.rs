//! 命令和查询处理器注册
//!
//! 提供统一的处理器注册函数，用于在应用启动时注册所有 CQRS 处理器

use std::sync::Arc;

use crate::application::commands::{
    handler::{CommandBus, InMemoryCommandBus},
    CreateTenantCommand, CreateTenantHandler,
    CreateOrganizationCommand, CreateOrganizationHandler,
    CreateTeamCommand, CreateTeamHandler,
    InviteMemberCommand, InviteMemberHandler,
    AcceptInviteCommand, AcceptInviteHandler,
    SuspendMemberCommand, SuspendMemberHandler,
};
use crate::application::queries::{
    handler::{QueryBus, InMemoryQueryBus},
    GetTenantQuery, GetTenantHandler,
    GetOrganizationQuery, GetOrganizationHandler,
    ListMembersQuery, ListMembersHandler,
    ListTeamsQuery, ListTeamsHandler,
};
use crate::domain::tenant::InMemoryTenantRepository;
use crate::domain::organization::InMemoryOrganizationRepository;
use crate::domain::team::InMemoryTeamRepository;
use crate::domain::member::InMemoryMembershipRepository;
use crate::domain::event::InMemoryEventPublisher;
use crate::domain::service::{
    TenantDomainService, OrganizationDomainService, TeamDomainService, MemberDomainService,
};

/// 注册所有命令和查询处理器
pub fn register_all_handlers(
    command_bus: &mut InMemoryCommandBus,
    query_bus: &mut InMemoryQueryBus,
) {
    // 创建共享的领域服务和仓库
    let tenant_repo = Arc::new(InMemoryTenantRepository::new());
    let org_repo = Arc::new(InMemoryOrganizationRepository::new());
    let team_repo = Arc::new(InMemoryTeamRepository::new());
    let member_repo = Arc::new(InMemoryMembershipRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // 创建领域服务
    let tenant_service = Arc::new(TenantDomainService::new(
        tenant_repo.clone(),
        event_publisher.clone(),
    ));
    let org_service = Arc::new(OrganizationDomainService::new(
        org_repo.clone(),
        event_publisher.clone(),
    ));
    let team_service = Arc::new(TeamDomainService::new(
        team_repo.clone(),
        event_publisher.clone(),
    ));
    let member_service = Arc::new(MemberDomainService::new(
        member_repo.clone(),
        event_publisher.clone(),
    ));

    // ========== 注册命令处理器 ==========

    // 租户命令
    command_bus.register_handler::<CreateTenantHandler<_, _>, CreateTenantCommand>(
        CreateTenantHandler::new(tenant_service.clone()),
    );

    // 组织命令
    command_bus.register_handler::<CreateOrganizationHandler<_, _>, CreateOrganizationCommand>(
        CreateOrganizationHandler::new(org_service.clone()),
    );

    // 团队命令
    command_bus.register_handler::<CreateTeamHandler<_, _>, CreateTeamCommand>(
        CreateTeamHandler::new(team_service.clone()),
    );

    // 成员命令 - InviteMemberHandler 没有泛型参数
    command_bus.register_handler::<InviteMemberHandler, InviteMemberCommand>(
        InviteMemberHandler::new(),
    );

    // 成员命令 - AcceptInviteHandler 和 SuspendMemberHandler 有泛型参数
    command_bus.register_handler::<AcceptInviteHandler<_, _>, AcceptInviteCommand>(
        AcceptInviteHandler::new(member_service.clone()),
    );
    command_bus.register_handler::<SuspendMemberHandler<_, _>, SuspendMemberCommand>(
        SuspendMemberHandler::new(member_service.clone()),
    );

    // ========== 注册查询处理器 ==========

    // 租户查询
    query_bus.register_handler::<GetTenantHandler<_, _>, GetTenantQuery>(
        GetTenantHandler::new(tenant_service.clone()),
    );

    // 组织查询
    query_bus.register_handler::<GetOrganizationHandler<_>, GetOrganizationQuery>(
        GetOrganizationHandler::new(org_repo.clone()),
    );

    // 成员查询
    query_bus.register_handler::<ListMembersHandler<_, _>, ListMembersQuery>(
        ListMembersHandler::new(member_service.clone()),
    );

    // 团队查询 - ListTeamsHandler 只有一个泛型参数
    query_bus.register_handler::<ListTeamsHandler<_>, ListTeamsQuery>(
        ListTeamsHandler::new(team_repo.clone()),
    );

    tracing::info!("All CQRS handlers registered successfully");
}
