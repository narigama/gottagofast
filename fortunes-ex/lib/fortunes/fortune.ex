defmodule Fortunes.Fortune do
  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id
  schema "fortune" do
    field :created_at, :utc_datetime
    field :updated_at, :utc_datetime
    field :content, :string
  end

  @doc false
  def changeset(fortune, attrs) do
    fortune
    |> cast(attrs, [])
    |> validate_required([])
  end
end
