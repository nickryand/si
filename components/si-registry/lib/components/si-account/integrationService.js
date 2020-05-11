"use strict";

var _registry = require("../../registry");

_registry.registry.system({
  typeName: "integrationService",
  displayTypeName: "An service within an integration",
  siPathName: "si-account",
  serviceName: "account",
  options: function options(c) {
    c.migrateable = true;
    c.associations.belongsTo({
      fromFieldPath: ["siProperties", "integrationId"],
      typeName: "integration"
    });
    c.fields.addObject({
      name: "siProperties",
      label: "SI Internal Properties",
      options: function options(p) {
        p.required = true;
        p.properties.addText({
          name: "integrationId",
          label: "Integration ID",
          options: function options(p) {
            p.required = true;
          }
        });
        p.properties.addNumber({
          name: "version",
          label: "The version of this integration",
          options: function options(p) {
            p.required = true;
            p.hidden = true;
            p.numberKind = "int32";
          }
        });
      }
    });
    c.addListMethod({
      isPrivate: true
    });
    c.addGetMethod();
    c.methods.addMethod({
      name: "create",
      label: "Create an Integration Servcie",
      options: function options(p) {
        p.mutation = true;
        p.hidden = true;
        p.isPrivate = true;
        p.request.properties.addText({
          name: "name",
          label: "Integration Service Name",
          options: function options(p) {
            p.required = true;
          }
        });
        p.request.properties.addText({
          name: "displayName",
          label: "Integration Service Display Name",
          options: function options(p) {
            p.required = true;
          }
        });
        p.request.properties.addLink({
          name: "siProperties",
          label: "Si Properties",
          options: function options(p) {
            p.required = true;
            p.lookup = {
              typeName: "integrationService",
              names: ["siProperties"]
            };
          }
        });
        p.reply.properties.addLink({
          name: "item",
          label: "".concat(c.displayTypeName, " Item"),
          options: function options(p) {
            p.lookup = {
              typeName: "integrationService"
            };
          }
        });
      }
    });
  }
});
//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIi4uLy4uLy4uL3NyYy9jb21wb25lbnRzL3NpLWFjY291bnQvaW50ZWdyYXRpb25TZXJ2aWNlLnRzIl0sIm5hbWVzIjpbInJlZ2lzdHJ5Iiwic3lzdGVtIiwidHlwZU5hbWUiLCJkaXNwbGF5VHlwZU5hbWUiLCJzaVBhdGhOYW1lIiwic2VydmljZU5hbWUiLCJvcHRpb25zIiwiYyIsIm1pZ3JhdGVhYmxlIiwiYXNzb2NpYXRpb25zIiwiYmVsb25nc1RvIiwiZnJvbUZpZWxkUGF0aCIsImZpZWxkcyIsImFkZE9iamVjdCIsIm5hbWUiLCJsYWJlbCIsInAiLCJyZXF1aXJlZCIsInByb3BlcnRpZXMiLCJhZGRUZXh0IiwiYWRkTnVtYmVyIiwiaGlkZGVuIiwibnVtYmVyS2luZCIsImFkZExpc3RNZXRob2QiLCJpc1ByaXZhdGUiLCJhZGRHZXRNZXRob2QiLCJtZXRob2RzIiwiYWRkTWV0aG9kIiwibXV0YXRpb24iLCJyZXF1ZXN0IiwiYWRkTGluayIsImxvb2t1cCIsIm5hbWVzIiwicmVwbHkiXSwibWFwcGluZ3MiOiI7O0FBTUE7O0FBR0FBLG1CQUFTQyxNQUFULENBQWdCO0FBQ2RDLEVBQUFBLFFBQVEsRUFBRSxvQkFESTtBQUVkQyxFQUFBQSxlQUFlLEVBQUUsa0NBRkg7QUFHZEMsRUFBQUEsVUFBVSxFQUFFLFlBSEU7QUFJZEMsRUFBQUEsV0FBVyxFQUFFLFNBSkM7QUFLZEMsRUFBQUEsT0FMYyxtQkFLTkMsQ0FMTSxFQUtXO0FBQ3ZCQSxJQUFBQSxDQUFDLENBQUNDLFdBQUYsR0FBZ0IsSUFBaEI7QUFFQUQsSUFBQUEsQ0FBQyxDQUFDRSxZQUFGLENBQWVDLFNBQWYsQ0FBeUI7QUFDdkJDLE1BQUFBLGFBQWEsRUFBRSxDQUFDLGNBQUQsRUFBaUIsZUFBakIsQ0FEUTtBQUV2QlQsTUFBQUEsUUFBUSxFQUFFO0FBRmEsS0FBekI7QUFJQUssSUFBQUEsQ0FBQyxDQUFDSyxNQUFGLENBQVNDLFNBQVQsQ0FBbUI7QUFDakJDLE1BQUFBLElBQUksRUFBRSxjQURX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Q0osZUFBUCxVQUZvQjtBQUd6QkcsVUFBQUEsT0FIeUIsbUJBR2pCVSxDQUhpQixFQUdKO0FBQ25CQSxZQUFBQSxDQUFDLENBQUNlLE1BQUYsR0FBVztBQUNUN0IsY0FBQUEsUUFBUSxFQUFFO0FBREQsYUFBWDtBQUdEO0FBUHdCLFNBQTNCO0FBU0Q7QUF6Q2lCLEtBQXBCO0FBMkNEO0FBakZhLENBQWhCIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0IHtcbiAgUHJvcE9iamVjdCxcbiAgUHJvcE1ldGhvZCxcbiAgUHJvcExpbmssXG4gIFByb3BOdW1iZXIsXG59IGZyb20gXCIuLi8uLi9jb21wb25lbnRzL3ByZWx1ZGVcIjtcbmltcG9ydCB7IHJlZ2lzdHJ5IH0gZnJvbSBcIi4uLy4uL3JlZ2lzdHJ5XCI7XG5pbXBvcnQgeyBTeXN0ZW1PYmplY3QgfSBmcm9tIFwiLi4vLi4vc3lzdGVtQ29tcG9uZW50XCI7XG5cbnJlZ2lzdHJ5LnN5c3RlbSh7XG4gIHR5cGVOYW1lOiBcImludGVncmF0aW9uU2VydmljZVwiLFxuICBkaXNwbGF5VHlwZU5hbWU6IFwiQW4gc2VydmljZSB3aXRoaW4gYW4gaW50ZWdyYXRpb25cIixcbiAgc2lQYXRoTmFtZTogXCJzaS1hY2NvdW50XCIsXG4gIHNlcnZpY2VOYW1lOiBcImFjY291bnRcIixcbiAgb3B0aW9ucyhjOiBTeXN0ZW1PYmplY3QpIHtcbiAgICBjLm1pZ3JhdGVhYmxlID0gdHJ1ZTtcblxuICAgIGMuYXNzb2NpYXRpb25zLmJlbG9uZ3NUbyh7XG4gICAgICBmcm9tRmllbGRQYXRoOiBbXCJzaVByb3BlcnRpZXNcIiwgXCJpbnRlZ3JhdGlvbklkXCJdLFxuICAgICAgdHlwZU5hbWU6IFwiaW50ZWdyYXRpb25cIixcbiAgICB9KTtcbiAgICBjLmZpZWxkcy5hZGRPYmplY3Qoe1xuICAgICAgbmFtZTogXCJzaVByb3BlcnRpZXNcIixcbiAgICAgIGxhYmVsOiBcIlNJIEludGVybmFsIFByb3BlcnRpZXNcIixcbiAgICAgIG9wdGlvbnMocDogUHJvcE9iamVjdCkge1xuICAgICAgICBwLnJlcXVpcmVkID0gdHJ1ZTtcbiAgICAgICAgcC5wcm9wZXJ0aWVzLmFkZFRleHQoe1xuICAgICAgICAgIG5hbWU6IFwiaW50ZWdyYXRpb25JZFwiLFxuICAgICAgICAgIGxhYmVsOiBcIkludGVncmF0aW9uIElEXCIsXG4gICAgICAgICAgb3B0aW9ucyhwKSB7XG4gICAgICAgICAgICBwLnJlcXVpcmVkID0gdHJ1ZTtcbiAgICAgICAgICB9LFxuICAgICAgICB9KTtcbiAgICAgICAgcC5wcm9wZXJ0aWVzLmFkZE51bWJlcih7XG4gICAgICAgICAgbmFtZTogXCJ2ZXJzaW9uXCIsXG4gICAgICAgICAgbGFiZWw6IFwiVGhlIHZlcnNpb24gb2YgdGhpcyBpbnRlZ3JhdGlvblwiLFxuICAgICAgICAgIG9wdGlvbnMocDogUHJvcE51bWJlcikge1xuICAgICAgICAgICAgcC5yZXF1aXJlZCA9IHRydWU7XG4gICAgICAgICAgICBwLmhpZGRlbiA9IHRydWU7XG4gICAgICAgICAgICBwLm51bWJlcktpbmQgPSBcImludDMyXCI7XG4gICAgICAgICAgfSxcbiAgICAgICAgfSk7XG4gICAgICB9LFxuICAgIH0pO1xuXG4gICAgYy5hZGRMaXN0TWV0aG9kKHsgaXNQcml2YXRlOiB0cnVlIH0pO1xuICAgIGMuYWRkR2V0TWV0aG9kKCk7XG4gICAgYy5tZXRob2RzLmFkZE1ldGhvZCh7XG4gICAgICBuYW1lOiBcImNyZWF0ZVwiLFxuICAgICAgbGFiZWw6IFwiQ3JlYXRlIGFuIEludGVncmF0aW9uIFNlcnZjaWVcIixcbiAgICAgIG9wdGlvbnMocDogUHJvcE1ldGhvZCkge1xuICAgICAgICBwLm11dGF0aW9uID0gdHJ1ZTtcbiAgICAgICAgcC5oaWRkZW4gPSB0cnVlO1xuICAgICAgICBwLmlzUHJpdmF0ZSA9IHRydWU7XG4gICAgICAgIHAucmVxdWVzdC5wcm9wZXJ0aWVzLmFkZFRleHQoe1xuICAgICAgICAgIG5hbWU6IFwibmFtZVwiLFxuICAgICAgICAgIGxhYmVsOiBcIkludGVncmF0aW9uIFNlcnZpY2UgTmFtZVwiLFxuICAgICAgICAgIG9wdGlvbnMocCkge1xuICAgICAgICAgICAgcC5yZXF1aXJlZCA9IHRydWU7XG4gICAgICAgICAgfSxcbiAgICAgICAgfSk7XG4gICAgICAgIHAucmVxdWVzdC5wcm9wZXJ0aWVzLmFkZFRleHQoe1xuICAgICAgICAgIG5hbWU6IFwiZGlzcGxheU5hbWVcIixcbiAgICAgICAgICBsYWJlbDogXCJJbnRlZ3JhdGlvbiBTZXJ2aWNlIERpc3BsYXkgTmFtZVwiLFxuICAgICAgICAgIG9wdGlvbnMocCkge1xuICAgICAgICAgICAgcC5yZXF1aXJlZCA9IHRydWU7XG4gICAgICAgICAgfSxcbiAgICAgICAgfSk7XG4gICAgICAgIHAucmVxdWVzdC5wcm9wZXJ0aWVzLmFkZExpbmsoe1xuICAgICAgICAgIG5hbWU6IFwic2lQcm9wZXJ0aWVzXCIsXG4gICAgICAgICAgbGFiZWw6IFwiU2kgUHJvcGVydGllc1wiLFxuICAgICAgICAgIG9wdGlvbnMocDogUHJvcExpbmspIHtcbiAgICAgICAgICAgIHAucmVxdWlyZWQgPSB0cnVlO1xuICAgICAgICAgICAgcC5sb29rdXAgPSB7XG4gICAgICAgICAgICAgIHR5cGVOYW1lOiBcImludGVncmF0aW9uU2VydmljZVwiLFxuICAgICAgICAgICAgICBuYW1lczogW1wic2lQcm9wZXJ0aWVzXCJdLFxuICAgICAgICAgICAgfTtcbiAgICAgICAgICB9LFxuICAgICAgICB9KTtcbiAgICAgICAgcC5yZXBseS5wcm9wZXJ0aWVzLmFkZExpbmsoe1xuICAgICAgICAgIG5hbWU6IFwiaXRlbVwiLFxuICAgICAgICAgIGxhYmVsOiBgJHtjLmRpc3BsYXlUeXBlTmFtZX0gSXRlbWAsXG4gICAgICAgICAgb3B0aW9ucyhwOiBQcm9wTGluaykge1xuICAgICAgICAgICAgcC5sb29rdXAgPSB7XG4gICAgICAgICAgICAgIHR5cGVOYW1lOiBcImludGVncmF0aW9uU2VydmljZVwiLFxuICAgICAgICAgICAgfTtcbiAgICAgICAgICB9LFxuICAgICAgICB9KTtcbiAgICAgIH0sXG4gICAgfSk7XG4gIH0sXG59KTtcbiJdfQ==