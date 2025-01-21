import datetime
import typing
import uuid

import sqlalchemy
from sqlalchemy.ext.asyncio import AsyncAttrs
from sqlalchemy.ext.asyncio import AsyncEngine
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine
from sqlalchemy.orm import DeclarativeBase
from sqlalchemy.orm import Mapped
from sqlalchemy.orm import mapped_column


def get_engine(database_url: str, pool_size: int) -> AsyncEngine:
    return create_async_engine(
        str(database_url).replace("postgres://", "postgresql+asyncpg://").replace("?sslmode=disable", ""),
        pool_size=pool_size,
        pool_recycle=3600,
    )


def create_session(engine: AsyncEngine) -> AsyncSession:
    return AsyncSession(engine)


Timestamp = typing.Annotated[datetime.datetime, mapped_column(sqlalchemy.DateTime(timezone=True))]


class Table(DeclarativeBase, AsyncAttrs):
    id: Mapped[uuid.UUID] = mapped_column(
        sqlalchemy.UUID,
        sort_order=-3,
        primary_key=True,
        server_default=sqlalchemy.text("gen_random_uuid()"),
    )

    created_at: Mapped[Timestamp] = mapped_column(
        server_default=sqlalchemy.text("now()"),
        sort_order=-2,
    )

    updated_at: Mapped[Timestamp] = mapped_column(
        server_default=sqlalchemy.text("now()"),
        sort_order=-1,
        onupdate=datetime.datetime.now(datetime.UTC),
    )


class Fortune(Table):
    __tablename__ = "fortune"

    content: Mapped[str]
