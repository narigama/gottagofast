import contextlib
import datetime
import functools
import multiprocessing
import uuid

import asyncpg
import fastapi
import pydantic
import pydantic_settings
from sqlalchemy import func
import sqlmodel
from sqlalchemy.ext.asyncio import AsyncEngine, create_async_engine
from sqlmodel.ext.asyncio.session import AsyncSession

MAX_CORES = multiprocessing.cpu_count()


Field = functools.partial(sqlmodel.Field, nullable=False)


def now() -> datetime.datetime:
    return datetime.datetime.now(tz=datetime.timezone.utc)


def get_engine(url: str) -> AsyncEngine:
    return create_async_engine(url=url, future=True)


def get_session(engine: AsyncEngine) -> AsyncSession:
    return AsyncSession(engine, expire_on_commit=False)


async def get_database(request: fastapi.Request) -> AsyncSession:
    """
    Mock this function in tests to avoid committing changes.
    """
    async with get_session(request.app.state.database_engine) as session, session.begin():
        yield session


class Config(pydantic_settings.BaseSettings):
    model_config = {"env_file": ".env"}
    database_url: pydantic.PostgresDsn


class Model:
    pass


class Fortune(sqlmodel.SQLModel, table=True):
    id: uuid.UUID = Field(primary_key=True, default_factory=uuid.uuid4)
    created_at: datetime.datetime = Field(default_factory=now)
    updated_at: datetime.datetime = Field(default_factory=now)
    content: str = Field()


class FortuneListRequest(pydantic.BaseModel):
    model_config = {
        "json_schema_extra": {
            "examples": [
                {
                    "quantity": 5,
                }
            ]
        }
    }

    quantity: int = pydantic.Field(5, gt=0, lt=21)


class FortuneListResponse(pydantic.BaseModel):
    count: int
    items: list[Fortune]


async def get_fortunes(db: AsyncSession, n: int) -> list[Fortune]:
    query = sqlmodel.select(Fortune).order_by(func.random()).limit(n)
    return (await db.exec(query)).all()


async def fortunes(
    request: FortuneListRequest,
    db: asyncpg.Connection = fastapi.Depends(get_database),
):
    """
    Return 5 fortunes at random. If quantity=N is passed via the query string,
    return that many items instead.
    """
    items = await get_fortunes(db, request.quantity)

    return FortuneListResponse(
        count=len(items),
        items=items,
    )


@contextlib.asynccontextmanager
async def lifespan(app: fastapi.FastAPI):
    app.state.database_engine = get_engine(
        str(app.state.config.database_url)
        .replace("postgres://", "postgresql+asyncpg://")
        .replace("?sslmode=disable", "")
    )
    yield
    # await app.state.database_pool.close()


def factory():
    app = fastapi.FastAPI(lifespan=lifespan)
    app.state.config = Config()
    app.add_api_route(path="/fortunes", endpoint=fortunes, methods=["POST"])

    return app
