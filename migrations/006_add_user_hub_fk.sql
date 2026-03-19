DO $$ BEGIN
    ALTER TABLE users
        ADD CONSTRAINT fk_users_home_hub
        FOREIGN KEY (home_hub_id) REFERENCES hubs(id)
        ON DELETE SET NULL;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
